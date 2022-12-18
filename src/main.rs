#[macro_use]
extern crate rocket;

use chrono::Datelike;
use rocket::fs::NamedFile;
use rocket::tokio::io::AsyncWriteExt;
// Static files
use rocket::fs::FileServer;

use log::{error, info, warn};

#[get("/?<day>&<month>&<gui>")]
async fn get_data(day: u8, month: u8, gui: bool) -> Result<NamedFile, String> {
    info!("Incoming request for {}.{} with GUI: {}", day, month, gui);
    if day > 31 || month > 12 {
        warn!("Invalid date: {}/{}", day, month);
        return Err("Invalid date".to_string());
    }
    // Make the day have 2 digits
    let day = format!("{:02}", day);
    let current_year = chrono::Local::now().year();
    let date = format!("{}.{}.{}", day, month, current_year);

    // Check if the file is in the cache
    let filename_pdf = format!("./cached/{}.pdf", date);
    if std::path::Path::new(&filename_pdf).exists() {
        // Check if the file is younger then 10 minutes
        let metadata = rocket::tokio::fs::metadata(&filename_pdf)
            .await
            .expect("Error while getting metadata");
        let file_age = chrono::Local::now()
            - chrono::DateTime::from(
                metadata
                    .modified()
                    .expect("Error while getting file modified date"),
            );
        if file_age.num_minutes() < 10 {
            // Return the file
            info!("Returning cached data for {}", date);
            return Ok(NamedFile::open(&filename_pdf)
                .await
                .expect("Error while opening file"));
        } else {
            // Delete the file
            info!("Deleting old cached data for {}", date);
            rocket::tokio::fs::remove_file(&filename_pdf)
                .await
                .expect("Error while deleting file");
            // And continue as normal
        }
    }

    info!("Getting data for {}", date);
    let response =
        match reqwest::get(format!("https://zastepstwa.zschie.pl/pliki/{}.pdf", date)).await {
            Ok(response) => response,
            Err(err) => return Err(format!("Error while fetching data: {}", err)),
        };

    // If the server returns a 200 status code
    if response.status() == 200 {
        // Create a new file
        let filename_pdf = format!("./cached/{}.pdf", date);

        let mut file = match rocket::tokio::fs::File::create(&filename_pdf).await {
            Ok(file) => file,
            Err(err) => return Err(format!("Error while creating file: {}", err)),
        };
        // Download the PDF
        let filebytes = match response.bytes().await {
            Ok(filebytes) => filebytes,
            Err(err) => return Err(format!("Error while converting file: {}", err)),
        };
        // Write the PDF to the file
        match file.write_all(&filebytes).await {
            Ok(file) => file,
            Err(err) => return Err(format!("Error while writing file: {}", err)),
        };

        // Return the PDF
        Ok(NamedFile::open(&filename_pdf)
            .await
            .expect("Error while opening file"))
    } else if response.status() == 404 {
        warn!("No data for {}", date);
        // If the server returns a 404 status code
        Err(format!(
            "Nie ma obecnie zastępstw na dzień {}! Spróbuj ponownie później!",
            date
        ))
    } else {
        // Return an error
        let response_status = response.status().as_u16();
        error!("Server returned a {} status code", response_status);
        Err(format!("Server returned a {} status code", response_status))
    }
}

#[get("/?<when>")]
async fn auto_get_data(when: String) -> Result<NamedFile, String> {
    // Get current date
    let current_date = if when == "tomorrow" {
        // If it's friday or saturday return message
        match chrono::Local::now().weekday() {
            chrono::Weekday::Fri => return Err("Jest jutro sobota, nie ma zastępstw!".to_string()),
            chrono::Weekday::Sat => {
                return Err("Jest jutro niedziela, nie ma zastępstw!".to_string())
            }
            _ => chrono::Local::now() + chrono::Duration::days(1),
        }
    } else if when == "today" {
        match chrono::Local::now().weekday() {
            chrono::Weekday::Sat => {
                return Err("Jest dziś sobota, nie ma dziś żadnych lekcji!".to_string())
            }
            chrono::Weekday::Sun => {
                return Err("Jest dziś niedziela, nie ma dziś żadnych lekcji!".to_string())
            }
            _ => chrono::Local::now(),
        }
    } else {
        return Err("Invalid type in request".to_string());
    };
    info!(
        "Incoming request for {} ({}.{})",
        when,
        current_date.day(),
        current_date.month()
    );

    // Format the current date to the PL format
    let date = current_date.format("%d.%m.%Y").to_string();
    // Send a get request to the server

    // Check if the file is in the cache
    let filename_pdf = format!("./cached/{}.pdf", date);
    if std::path::Path::new(&filename_pdf).exists() {
        // Check if the file is younger then 10 minutes
        let metadata = rocket::tokio::fs::metadata(&filename_pdf)
            .await
            .expect("Error while getting metadata");
        let file_age = chrono::Local::now()
            - chrono::DateTime::from(
                metadata
                    .modified()
                    .expect("Error while getting file modified date"),
            );
        if file_age.num_minutes() < 10 {
            // Return the file
            info!("Returning cached data for {}", date);
            return Ok(NamedFile::open(&filename_pdf)
                .await
                .expect("Error while opening file"));
        } else {
            // Delete the file
            info!("Deleting old cached data for {}", date);
            rocket::tokio::fs::remove_file(&filename_pdf)
                .await
                .expect("Error while deleting file");
            // And continue as normal
        }
    }

    info!("Getting data for {}", date);
    let response =
        match reqwest::get(format!("https://zastepstwa.zschie.pl/pliki/{}.pdf", date)).await {
            Ok(response) => response,
            Err(err) => return Err(format!("Error while fetching data: {}", err)),
        };

    // If the server returns a 200 status code
    if response.status() == 200 {
        // Create a new file
        let filename_pdf = format!("./cached/{}.pdf", date);

        let mut file = match rocket::tokio::fs::File::create(&filename_pdf).await {
            Ok(file) => file,
            Err(err) => return Err(format!("Error while creating file: {}", err)),
        };
        // Download the PDF
        let filebytes = match response.bytes().await {
            Ok(filebytes) => filebytes,
            Err(err) => return Err(format!("Error while converting file: {}", err)),
        };
        // Write the PDF to the file
        match file.write_all(&filebytes).await {
            Ok(file) => file,
            Err(err) => return Err(format!("Error while writing file: {}", err)),
        };

        // Return the file
        match NamedFile::open(&filename_pdf).await {
            Ok(file) => Ok(file),
            Err(err) => Err(format!("Error while opening file: {}", err)),
        }
    } else if response.status() == 404 {
        // If the server returns a 404 status code
        warn!("No data for {}", date);
        Err(format!(
            "Nie ma obecnie zastępstw na dzień {}! Spróbuj ponownie później!",
            date
        ))
    } else {
        // Return an error
        let response_status = response.status().as_u16();
        error!("Server returned a {} status code", response_status);
        Err(format!("Server returned a {} status code", response_status))
    }
}

// Status page
#[get("/")]
async fn status() -> &'static str {
    "Strona jest online!"
}

// 404 handler
#[catch(404)]
async fn not_found() -> &'static str {
    "Nie ma takiej strony! Jeśli to błąd, napisz do twórcy."
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
    // Check if the log file exists
    if !std::path::Path::new("./log.yml").exists() {
        // If it doesn't, create it
        rocket::tokio::fs::File::create("./log.yml")
            .await
            .expect("Error while creating log file");
        // And write the default config to it
        // https://tms-dev-blog.com/log-to-a-file-in-rust-with-log4rs/
        let config = r#"
        appenders:  
            my_stdout:
                kind: console
                encoder:
                    pattern: "{h({d(%Y-%m-%d %H:%M:%S)(utc)} - {l}: {m}{n})}"
            my_file_logger:
                kind: rolling_file
                path: "log/log.log"
                encoder:
                    pattern: "{d(%Y-%m-%d %H:%M:%S)(utc)} - {h({l})}: {m}{n}"
                policy:
                    trigger:
                        kind: size
                        limit: 50kb
                    roller:
                        kind: fixed_window
                        base: 1
                        count: 10
                        pattern: "log/log{}.log"
        root:
            level: info
            appenders:
                - my_stdout
                - my_file_logger"#;
        rocket::tokio::fs::write("./log.yml", config)
            .await
            .expect("Error while writing to log file");
    }
    // Set the logger
    log4rs::init_file("./log.yml", Default::default()).expect("Error while setting logger");

    // Check if the Rocket.toml file exists
    if !std::path::Path::new("./Rocket.toml").exists() {
        // If it doesn't, create it
        rocket::tokio::fs::File::create("./Rocket.toml")
            .await
            .expect("Error while creating Rocket.toml file");
        // And write the default config to it
        let config = r#"
        [global]
        address = \"0.0.0.0\"
        port = 9000
        log_level = \"off\"
        "#;
        rocket::tokio::fs::write("./Rocket.toml", config)
            .await
            .expect("Error while writing to Rocket.toml file");
    }
    
    // Start the server
    rocket::build()
        // Static files
        .mount("/public", FileServer::from("static/"))
        .mount("/", routes![get_data])
        .mount("/auto/", routes![auto_get_data])
        .mount("/status/", routes![status])
        .register("/", catchers![not_found])
}
