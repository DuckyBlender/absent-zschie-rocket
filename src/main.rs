#[macro_use]
extern crate rocket;

use chrono::Datelike;
use log::{error, info, warn};
use rocket::fs::NamedFile;
use rocket::tokio::io::AsyncWriteExt;
use serde_json::{json, Value};

const CACHE_TIME_MIN: i64 = 30;
const DOMAIN: &str = "https://zastepstwa.ducky.pics";

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
    if date.unwrap().weekday() == chrono::Weekday::Sat
        || date.unwrap().weekday() == chrono::Weekday::Sun
    {
        // If it is, return an error
        warn!("Date on weekend: {}.{}", day, month);
        return json!({"code": 422, "error": "Wybrana data to weekend!"});
    }

    // Setup common variables
    let date = format!("{}.{}.{}", day, month, year);
    let filename_pdf = format!("./cached/{}.pdf", date);

    // Check if the file already exists in the cache
    if std::path::Path::new(&filename_pdf).exists() {
        // Check if the file is younger then X minutes
        let metadata = rocket::tokio::fs::metadata(&filename_pdf)
            .await
            .expect("Error while getting metadata");
        let file_age = chrono::Local::now()
            - chrono::DateTime::from(
                metadata
                    .modified()
                    .expect("Error while getting file modified date"),
            );

        // If the file is younger than X minutes, stop the function
        if file_age.num_minutes() < CACHE_TIME_MIN {
            // If it is, stop the function
            info!("Using cached data for {}", date);
            return json!({
                "code": 200,
                "link": format!("{}/files/{}.pdf", DOMAIN, date)
            });
        // If it isn't, delete the file and download the new one
        } else {
            // Delete the file
            info!(
                "Deleting old cached data for {}. It is {} minutes old, the max is {}",
                date,
                file_age.num_minutes(),
                CACHE_TIME_MIN
            );
            rocket::tokio::fs::remove_file(&filename_pdf)
                .await
                .expect("Error while deleting file");
            // And continue the function
        }
    }

    // Get the new data
    info!("Getting new data for {}", date);
    let response = reqwest::get(format!("https://zastepstwa.zschie.pl/pliki/{}.pdf", date))
        .await
        .unwrap();
    // If the server returns a 200 status code
    match response.status().as_u16() {
        200 => {
            // Create a new file
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
            // If the server returns a 404 status code
            warn!("Nie ma zastępstw na dzień {}", date);
            json!({
                "code": 404,
                "error": format!("Nie ma zastępstw na dzień {}. Spróbuj ponownie później!", date)
            })
        }
        _ => {
            // Return an error if the server returns a different status code
            let response_status = response.status().as_u16();
            error!("Server returned a {} status code", response_status);
            json!({
                "code": 500,
                "error": format!("Server zwrócił nieznany status {}. Spróbuj ponownie później!", response_status)
            })
        }
    }
}

#[get("/?<day>&<month>&<year>")]
async fn get_data(day: u32, month: u32, year: i32) -> Result<Value, Value> {
    info!("Incoming request for {}.{}", day, month);

    let json = ready_file(day, month, year).await;
    Ok(json)
}

#[get("/?<when>")]
async fn auto_get_data(when: String) -> Result<Value, Value> {
    // Get current date
    let (day, month, year): (u32, u32, i32) = match when.as_str() {
        "today" => match chrono::Local::now().weekday() {
            chrono::Weekday::Sat => {
                return Err(json!({"code": 422, "error": "Jest dziś sobota, nie ma dziś żadnych lekcji!"}))
            }
            chrono::Weekday::Sun => {
                return Err(json!({"code": 422, "error": "Jest dziś niedziela, nie ma dziś żadnych lekcji!"}))
            }
            _ => {
                let date = chrono::Local::now().naive_local().date();
                (date.day(), date.month(), date.year())
            }
        },
        "tomorrow" => match chrono::Local::now().weekday() {
            chrono::Weekday::Fri => {
                return Err(json!({"code": 422, "error": "Jest jutro sobota, więc nie ma zastępstw!"}))
            }
            chrono::Weekday::Sat => {
                return Err(json!({"code": 422, "error": "Jest jutro niedziela, więc nie ma zastępstw!"}))
            }
            _ => {
                let date = chrono::Local::now().naive_local().date() + chrono::Duration::days(1);
                (date.day(), date.month(), date.year())
            }
        },
        _ => {
            error!("Invalid parameter for when: {}", when);
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

// 404 handler
#[catch(404)]
async fn not_found() -> &'static str {
    "Nie ma takiej strony! Jeśli uważasz że to błąd, napisz do twórcy."
}

// 500 handler
#[catch(500)]
async fn internal_server_error() -> &'static str {
    "Wystąpił błąd wewnętrzny!"
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
        // Static files
        .mount("/", routes![get_data])
        .mount("/auto/", routes![auto_get_data])
        .mount("/status/", routes![status])
        .mount("/files/", routes![files])
        .register("/", catchers![not_found, internal_server_error])
}
