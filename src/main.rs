#[macro_use]
extern crate rocket;

use chrono::Datelike;
use log::{error, info, warn};
use rocket::fs::NamedFile;
use rocket::tokio::io::AsyncWriteExt;
use serde_json::{json, Value};

const CACHE_TIME_MIN: i64 = 10;

async fn ready_file(day: u8, month: u8, year: u32) -> Value {
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
                "status": "ok",
                "link": format!("https://ducky.pics/files/{}.pdf", date)
            });
        // If it isn't, delete the file and download the new one
        } else {
            // Delete the file
            info!("Deleting old cached data for {}. It is {} minutes old, the max is {}", date, file_age.num_minutes(), CACHE_TIME_MIN);
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
            file.flush()
                .await
                .expect("Error while flushing file");
            // Return the link to the file
            info!("Saved new data for {}", date);
            return json!({
                "status": "ok",
                "link": format!("https://ducky.pics/files/{}.pdf", date)
            });
        }
        404 => {
            // If the server returns a 404 status code
            warn!("Nie ma zastępstw na dzień {}", date);
            return json!({
                "status": "not_found",
                "error": format!("Nie ma zastępstw na dzień {}. Spróbuj ponownie później!", date)
            });
        }
        _ => {
            // Return an error if the server returns a different status code
            let response_status = response.status().as_u16();
            error!("Server returned a {} status code", response_status);
            return json!({
                "status": "error",
                "error": format!("Server zwrócił nieznany status {}. Spróbuj ponownie później!", response_status)
            });
        }
    }
}

#[get("/?<day>&<month>")]
async fn get_data(day: u8, month: u8) -> Result<Value, Value> {
    info!("Incoming request for {}.{}", day, month);
    if day > 31 || month > 12 {
        warn!("Invalid date: {}/{}", day, month);
        return Err(json!({
            "error": "Invalid date"
        }));
    }
    // Make the day have 2 digits
    let day: u8 = format!("{:02}", day).parse().unwrap();
    let current_year: u32 = chrono::Local::now().year().try_into().unwrap();

    ready_file(day, month, current_year).await;
    let link = format!(
        "https://ducky.pics/files/{}.{}.{}.pdf",
        day, month, current_year
    );
    Ok(json!({
        "status": "ok",
        "link": link
    }))
}

#[get("/?<when>")]
async fn auto_get_data(when: String) -> Result<Value, Value> {
    // Get current date
    let current_date = if when == "tomorrow" {
        // If it's friday or saturday return message
        match chrono::Local::now().weekday() {
            chrono::Weekday::Fri => {
                return Err(json!({"error": "Jest jutro sobota, więc nie ma zastępstw!"}))
            }
            chrono::Weekday::Sat => {
                return Err(json!({"error": "Jest jutro niedziela, więc nie ma zastępstw!"}))
            }
            _ => chrono::Local::now() + chrono::Duration::days(1),
        }
    } else if when == "today" {
        match chrono::Local::now().weekday() {
            chrono::Weekday::Sat => {
                return Err(json!({"error": "Jest dziś sobota, nie ma dziś żadnych lekcji!"}))
            }
            chrono::Weekday::Sun => {
                return Err(json!({"error": "Jest dziś niedziela, nie ma dziś żadnych lekcji!"}))
            }
            _ => chrono::Local::now(),
        }
    } else {
        error!("Invalid parameter for when: {}", when);
        return Err(json!({"error": "Niepoprawny parametr!"}));
    };
    info!(
        "Incoming request for {} ({}.{})",
        when,
        current_date.day(),
        current_date.month()
    );

    // Make the day have 2 digits
    let day: u8 = format!("{:02}", current_date.day()).parse().unwrap();
    let month: u8 = format!("{:02}", current_date.month()).parse().unwrap();
    let current_year: u32 = current_date.year().try_into().unwrap();

    ready_file(day, month, current_year).await;

    let link = format!(
        "https://ducky.pics/files/{}.{}.{}.pdf",
        day, month, current_year
    );

    Ok(json!({
        "status": "ok",
        "link": link
    }))
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
    "Wystąpił błąd serwera! Jeśli uważasz że to błąd, napisz do twórcy."
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
