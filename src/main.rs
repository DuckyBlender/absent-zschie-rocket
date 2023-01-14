#[macro_use]
extern crate rocket;

use chrono::Datelike;
use chrono::{DateTime, Weekday};
use log::{error, info, warn};
use rocket::fs::NamedFile;
use rocket::tokio::io::AsyncWriteExt;
use serde_json::{json, Value};

const CACHE_TIME_MIN: i64 = 30;
const DOMAIN: &str = "https://zastepstwa.ducky.pics";
// Uncomment for local testing
// const DOMAIN: &str = "http://192.168.1.253:5000";

async fn ready_file(day: u32, month: u32, year: i32) -> Value {
    // Check if the date is valid using chrono
    if chrono::NaiveDate::from_ymd_opt(year, month, day).is_none() {
        // If it isn't, return an error
        warn!("Invalid date: {}.{}", day, month);
        return json!({"code": 422, "error": "Nieprawidłowa data!"});
    }
    // Make the day have 2 digits
    let day = format!("{:02}", day);
    // Make the month have 2 digits
    let month = format!("{:02}", month);

    // Check if the date is on the weekend
    let date = chrono::NaiveDate::from_ymd_opt(year, month.parse().unwrap(), day.parse().unwrap());
    if date.unwrap().weekday() == Weekday::Sat || date.unwrap().weekday() == Weekday::Sun {
        // If it is, return an error
        warn!("Date on weekend: {}.{}", day, month);
        return json!({"code": 422, "error": "Wybrana data to weekend!"});
    }

    // Setup common variables
    let date = format!("{}.{}.{}", day, month, year);
    let filename_pdf = format!("./cached/{}.pdf", date);

    // Check if the file already exists in the cache
    if std::path::Path::new(&filename_pdf).exists() {
        // Check if the file is younger then X minutes. If it is, return the link to the file. If it isn't, try to download the new one.
        // If it fails, return the link to the old file. If it succeeds, return the link to the new file and delete the old one.
        let metadata = rocket::tokio::fs::metadata(&filename_pdf)
            .await
            .expect("Error while getting metadata");
        let file_age = chrono::Local::now()
            - DateTime::from(
                metadata
                    .modified()
                    .expect("Error while getting file modified date"),
            );

        // If the file is younger than X minutes, return the link to the file
        if file_age.num_minutes() < CACHE_TIME_MIN {
            // If it is, return the link to the file
            info!("Using cached data for {}, because the file is fresh", date);
            return json!({
                "code": 200,
                "link": format!("{}/files/{}.pdf", DOMAIN, date)
            });
        }
    }

    // If we got here, it means that the file doesn't exist or it's too old. We need to download the new one.
    info!("Getting new data for {}", date);
    let response = match reqwest::get(format!("https://zastepstwa.zschie.pl/pliki/{}.pdf", date))
        .await
    {
        Ok(response) => response,
        Err(_) => {
            error!("Error while downloading data for {}", date);
            // If the file exists, return the link to the old file
            if std::path::Path::new(&filename_pdf).exists() {
                // If it does, return the link to the file
                info!(
                        "Using cached data for {}, because the file existed and cannot get new one, because the server is offline",
                        date
                    );
                return json!({
                    "code": 200,
                    "link": format!("{}/files/{}.pdf", DOMAIN, date)
                });
            }
            return json!({
                "code": 500,
                "error": "Strona szkoły jest offline! Spróbuj ponownie później!"
            });
        }
    };

    // Match different status codes from the server and act accordingly
    match response.status().as_u16() {
        200 => {
            // Create the file, but first we need to make sure that the file doesn't exist
            if std::path::Path::new(&filename_pdf).exists() {
                // If it does, delete it
                rocket::tokio::fs::remove_file(&filename_pdf)
                    .await
                    .expect("Error while deleting file");
            }
            let mut file = rocket::tokio::fs::File::create(&filename_pdf)
                .await
                .expect("Error while creating file");
            // Download the PDF
            let filebytes = response.bytes().await.unwrap();
            // Write the PDF to the file
            file.write_all(&filebytes)
                .await
                .expect("Error while writing file");
            // Close the file
            file.flush().await.expect("Error while flushing file");
            // Return the link to the file
            info!("Saved new data for {}", date);
            json!({
                "code": 200,
                "link": format!("{}/files/{}.pdf", DOMAIN, date)
            })
        }
        404 => {
            // If the server returns a 404 status code, it means that there are currently no substitutions available
            // Check if the file exists
            if std::path::Path::new(&filename_pdf).exists() {
                // If it does, return the link to the file
                info!(
                    "Using cached data for {}, because the file existed and cannot get new one, because there are no substitutions",
                    date
                );
                json!({
                    "code": 200,
                    "link": format!("{}/files/{}.pdf", DOMAIN, date)
                })
            } else {
                // If it doesn't, return an error
                warn!("There are no substitutions for {}", date);
                json!({
                    "code": 404,
                    "error": format!("Nie ma zastępstw na dzień {}. Spróbuj ponownie później!", date)
                })
            }
        }
        _ => {
            // Return an error if the server returns a different status code
            let response_status = response.status().as_u16();
            error!("Server returned a {} status code", response_status);
            // Check if the file exists
            if std::path::Path::new(&filename_pdf).exists() {
                // If it does, return the link to the file
                info!(
                    "Using cached data for {}, because the server returned an unknown status code",
                    date
                );
                json!({
                    "code": 200,
                    "link": format!("{}/files/{}.pdf", DOMAIN, date)
                })
            } else {
                // If it doesn't, return an error
                warn!(
                    "Server returned an unknown status code: {}",
                    response_status
                );
                json!({
                    "code": 404,
                    "error": format!("Server zwrócił nieznany status {}. Spróbuj ponownie później!", response_status)
                })
            }
        }
    }
}

// /?day=1&month=1&year=2021
#[get("/?<day>&<month>&<year>")]
async fn get_data(day: u32, month: u32, year: i32) -> Result<Value, Value> {
    info!("Incoming request for {:02}.{:02}.{}", day, month, year);

    let json = ready_file(day, month, year).await;
    Ok(json)
}

// /auto/?when=today
// /auto/?when=tomorrow
#[get("/?<when>")]
async fn auto_get_data(when: String) -> Result<Value, Value> {
    // Get current date
    let (day, month, year): (u32, u32, i32) = match when.as_str() {
        "today" => match chrono::Local::now().weekday() {
            Weekday::Sat => {
                return Err(
                    json!({"code": 422, "error": "Jest dziś sobota, nie ma dziś żadnych lekcji!"}),
                )
            }
            Weekday::Sun => {
                return Err(
                    json!({"code": 422, "error": "Jest dziś niedziela, nie ma dziś żadnych lekcji!"}),
                )
            }
            _ => {
                let date = chrono::Local::now().naive_local().date();
                (date.day(), date.month(), date.year())
            }
        },
        "tomorrow" => match chrono::Local::now().weekday() {
            Weekday::Fri => {
                return Err(
                    json!({"code": 422, "error": "Jest jutro sobota, więc nie ma zastępstw!"}),
                )
            }
            Weekday::Sat => {
                return Err(
                    json!({"code": 422, "error": "Jest jutro niedziela, więc nie ma zastępstw!"}),
                )
            }
            _ => {
                let date = chrono::Local::now().naive_local().date() + chrono::Duration::days(1);
                (date.day(), date.month(), date.year())
            }
        },
        _ => {
            warn!("Invalid parameter for when: {}", when);
            return Err(json!({
                "code": 422,
                "error": "Nieprawidłowa data!"}));
        }
    };

    let json = ready_file(day, month, year).await;

    Ok(json)
}

// File serving (for example, localhost:9000/files/10.10.2022.pdf)
#[get("/<file>")]
async fn files(file: &str) -> NamedFile {
    NamedFile::open(format!("./cached/{}", file))
        .await
        .expect("Error while opening file")
}

// Status page
#[get("/")]
async fn status() -> &'static str {
    "Strona jest online!"
}

// Statistics page
#[get("/stats")]
async fn stats() -> Value {
    // Get the number of files in the cached folder
    let files = std::fs::read_dir("./cached")
        .expect("Error while reading cached folder")
        .count();
    json!({ "files": files })
}

// 404 handler
#[catch(404)]
async fn not_found() -> &'static str {
    "Nie ma takiej strony! Jeśli uważasz że to błąd, napisz do twórcy."
}

#[launch]
async fn launch() -> _ {
    // Check if the cached folder exists
    if !std::path::Path::new("./cached").exists() {
        // If it doesn't, create it
        rocket::tokio::fs::create_dir("./cached")
            .await
            .expect("Error while creating cached folder");
    }
    // Don't check for the log or config file, because they are in the Github repo

    // Start the server
    rocket::build()
        // Main routes
        .mount("/", routes![get_data, stats])
        .mount("/auto/", routes![auto_get_data])
        // Status route
        .mount("/status/", routes![status])
        // File serving route
        .mount("/files/", routes![files])
        // Error handlers
        .register("/", catchers![not_found])
}
