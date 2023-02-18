use actix_files::NamedFile;
use actix_web::web;
use actix_web::{get, middleware::Logger, App, HttpServer};
use actix_web::{HttpResponse, Responder};
use chrono::Datelike;
use chrono::{DateTime, Weekday};
use log::{error, info, warn};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::AsyncWriteExt;

const CACHE_TIME_MIN: i64 = 30;
const DOMAIN: &str = "https://zastepstwa.ducky.pics";
// Uncomment for local testing
// const DOMAIN: &str = "http://192.168.1.253:5000";
// const DOMAIN: &str = "http://172.20.10.3:5000";
const MAINTENENCE: bool = true;
// JSON Response Struct
#[derive(Serialize)]
struct Response {
    code: u16,
    link: String,
}

// Date Struct
#[derive(Serialize, Deserialize)]
struct Date {
    day: u32,
    month: u32,
    year: i32,
}

async fn ready_file(day: u32, month: u32, year: i32) -> Value {
    if MAINTENENCE {
        return json!({"code": 500, "error": "Skrót jest tymczasowo niedostępny przez zmiany w systemie szkoły. Próbujemy znaleźć rozwiązanie problemu. Nie będzie wymagać aktualizacji skrótu."});
    }

    // Check if the date is valid using chrono
    if chrono::NaiveDate::from_ymd_opt(year, month, day).is_none() {
        // If it isn't, return an error
        warn!("Invalid date: {}.{}", day, month);
        return json!({"code": 422, "error": "Nieprawidłowa data!"});
    }

    // Check if the date is on the winter break (16.01 - 29.01)
    if month == 1 && (16..=29).contains(&day) {
        // If it is, return an error
        warn!("Date on winter break: {}.{}", day, month);
        return json!({"code": 422, "error": "Jest przerwa zimowa! Możesz odpoczywać!"});
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
        let metadata = tokio::fs::metadata(&filename_pdf)
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
                tokio::fs::remove_file(&filename_pdf)
                    .await
                    .expect("Error while deleting file");
            }
            let mut file = tokio::fs::File::create(&filename_pdf)
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
#[get("/")]
async fn get_data(date: web::Query<Date>) -> impl Responder {
    let (day, month, year) = (date.day, date.month, date.year);
    info!("Incoming request for {:02}.{:02}.{}", day, month, year);
    let json = ready_file(day, month, year).await;
    HttpResponse::Ok()
        .content_type("application/json")
        .body(json.to_string())
}

#[derive(Deserialize)]
struct When {
    when: String,
}

// /auto/?when=today
// /auto/?when=tomorrow
#[get("/auto/")]
async fn auto_get_data(when: web::Query<When>) -> impl Responder {
    let when = when.when.to_lowercase();
    if when != "today" && when != "tomorrow" {
        let json = json!({"code": 422, "error": "Nieprawidłowa wartość parametru 'when'"});
        return HttpResponse::build(StatusCode::UNPROCESSABLE_ENTITY)
            .content_type("application/json")
            .body(json.to_string());
    }
    info!("Incoming request for {}", when);
    if MAINTENENCE {
        let json = json!({"code": 500, "error": "Skrót jest tymczasowo niedostępny. Próbujemy znaleźć rozwiązanie problemu."});
        return HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR)
            .content_type("application/json")
            .body(json.to_string());
    }
    // Get current date
    let (day, month, year): (u32, u32, i32) = match when.as_str() {
        "today" => match chrono::Local::now().weekday() {
            Weekday::Sat => {
                let json =
                    json!({"code": 422, "error": "Jest dziś sobota, nie ma dziś żadnych lekcji!"});
                return HttpResponse::build(StatusCode::UNPROCESSABLE_ENTITY)
                    .content_type("application/json")
                    .body(json.to_string());
            }
            Weekday::Sun => {
                let json = json!({"code": 422, "error": "Jest dziś niedziela, nie ma dziś żadnych lekcji!"});
                return HttpResponse::build(StatusCode::UNPROCESSABLE_ENTITY)
                    .content_type("application/json")
                    .body(json.to_string());
            }
            _ => {
                let date = chrono::Local::now().naive_local().date();
                (date.day(), date.month(), date.year())
            }
        },
        "tomorrow" => match chrono::Local::now().weekday() {
            Weekday::Fri => {
                let json =
                    json!({"code": 422, "error": "Jutro jest sobota, więc nie ma zastępstw!"});
                return HttpResponse::build(StatusCode::UNPROCESSABLE_ENTITY)
                    .content_type("application/json")
                    .body(json.to_string());
            }
            Weekday::Sat => {
                let json =
                    json!({"code": 422, "error": "Jutro jest niedziela, więc nie ma zastępstw!"});
                return HttpResponse::build(StatusCode::UNPROCESSABLE_ENTITY)
                    .content_type("application/json")
                    .body(json.to_string());
            }
            _ => {
                let date = chrono::Local::now().naive_local().date() + chrono::Duration::days(1);
                (date.day(), date.month(), date.year())
            }
        },
        _ => {
            warn!("Invalid parameter for when: {}", when);
            let json = json!({"code": 422, "error": "Nieprawidłowa wartość parametru 'when'"});
            return HttpResponse::build(StatusCode::UNPROCESSABLE_ENTITY)
                .content_type("application/json")
                .body(json.to_string());
        }
    };

    let json = ready_file(day, month, year).await;

    HttpResponse::Ok()
        .content_type("application/json")
        .body(json.to_string())
}

// File serving (for example, localhost:5000/files/10.10.2022.pdf)
#[get("/files/{day}.{month}.{year}.pdf")]
async fn files(file: web::Path<Date>) -> NamedFile {
    let file = file.into_inner();
    let file = format!("{}.{}.{}.pdf", file.day, file.month, file.year);
    // Check if the file exists
    if !std::path::Path::new(&format!("./cached/{}", file)).exists() {
        // If it doesn't, return an error
        warn!("File {} does not exist", file);
        return NamedFile::open_async("./pdf/brak.pdf")
            .await
            .expect("Error while opening file");
    }

    NamedFile::open_async(format!("./cached/{}", file))
        .await
        .expect("Error while opening file")
}

// Status page
#[get("/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

// Statistics page
#[get("/stats")]
async fn stats() -> impl Responder {
    // Get the number of files in the cached folder
    let filecount = std::fs::read_dir("./cached").unwrap().count();
    // Return the number of files
    let json = json!({ "files": filecount });
    HttpResponse::Ok().json(json)
}

// getpdf route for making this work as fast as possible using one request only. if it fails just return an error pdf located in the pdf/brak.pdf directory. used for the android app
#[get("/getpdf")]
async fn getpdf(date: web::Query<Date>) -> NamedFile {
    // This strictly returns a PDF file, not JSON
    let (day, month, year) = (date.day, date.month, date.year);
    info!("Incoming FAST request for {:02}.{:02}.{}", day, month, year);
    let res = ready_file(day, month, year).await;
    // Check if the request was successful
    if res["code"] == 200 {
        // If it was, return the file, get it from the cached folder using the date
        let date = format!("{:02}.{:02}.{}", day, month, year);
        let file = match NamedFile::open_async(format!("./cached/{}.pdf", date)).await {
            Ok(file) => file,
            Err(_) => {
                error!("Error while opening file {}", date);
                return NamedFile::open_async("./pdf/brak.pdf")
                    .await
                    .expect("Error while opening file");
            }
        };
        file
    } else {
        // If it wasn't, return the error pdf
        NamedFile::open_async("./pdf/brak.pdf").await.unwrap()
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Setup variables
    std::env::set_var("RUST_LOG", "dev");
    std::env::set_var("RUST_BACKTRACE", "1");
    // Set up logging
    env_logger::init();
    // Set up the cache folder if it doesn't exist. Asynchronous because why not
    tokio::fs::create_dir_all("./cached").await.unwrap();
    // Create the server
    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(get_data)
            .service(auto_get_data)
            .service(files)
            .service(status)
            .service(stats)
            .service(getpdf)
    })
    .bind(("0.0.0.0", 5000))?
    .run()
    .await
}
