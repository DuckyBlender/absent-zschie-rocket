#[macro_use]
extern crate rocket;

use chrono::Datelike;
// import json from rocket
use rocket::fs::NamedFile;
use rocket::tokio::io::AsyncWriteExt;

#[get("/?<day>&<month>")]
async fn get_data(day: u8, month: u8) -> Result<NamedFile, String> {
    if day > 31 || month > 12 {
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
        let metadata = std::fs::metadata(&filename_pdf).unwrap();
        let file_age = chrono::Local::now() - chrono::DateTime::from(metadata.modified().unwrap());
        if file_age.num_minutes() < 10 {
            // Return the file
            return Ok(NamedFile::open(&filename_pdf).await.unwrap());
        }
        else {
            // Delete the file
            std::fs::remove_file(&filename_pdf).unwrap();
            // And continue as normal
        }
    }

    let response = match reqwest::get(format!(
        "https://zastepstwa.zschie.pl/pliki/{}.pdf",
        date
    ))
    .await
    {
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
        Ok(NamedFile::open(&filename_pdf).await.unwrap())
    } else if response.status() == 404 {
        // If the server returns a 404 status code
        Err(format!(
            "Nie ma obecnie zastępstw na dzień {}! Spróbuj ponownie później!",
            date
        ))
    } else {
        // Return an error
        let response_status = response.status().as_u16();
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
            chrono::Weekday::Sat => return Err("Jest jutro niedziela, nie ma zastępstw!".to_string()),
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

    // Format the current date to the PL format
    let date = current_date.format("%d.%m.%Y").to_string();
    // Send a get request to the server

    // Check if the file is in the cache
    let filename_pdf = format!("./cached/{}.pdf", date);
    if std::path::Path::new(&filename_pdf).exists() {
        // Check if the file is younger then 10 minutes
        let metadata = std::fs::metadata(&filename_pdf).unwrap();
        let file_age = chrono::Local::now() - chrono::DateTime::from(metadata.modified().unwrap());
        if file_age.num_minutes() < 10 {
            // Return the file
            return Ok(NamedFile::open(&filename_pdf).await.unwrap());
        }
        else {
            // Delete the file
            std::fs::remove_file(&filename_pdf).unwrap();
            // And continue as normal
        }
    }
    let response = match reqwest::get(format!(
        "https://zastepstwa.zschie.pl/pliki/{}.pdf",
        date
    ))
    .await
    {
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
        Err(format!(
            "Nie ma obecnie zastępstw na dzień {}! Spróbuj ponownie później!",
            date
        ))
    } else {
        // Return an error
        let response_status = response.status().as_u16();
        Err(format!("Server returned a {} status code", response_status))
    }
}

// 404 handler
#[catch(404)]
fn not_found() -> &'static str {
    "Nie ma takiej strony! Napisz do twórcy jeśli uważasz, że to błąd!"
}

#[launch]
fn launch() -> _ {
    // Check if the cached folder exists
    if !std::path::Path::new("./cached").exists() {
        // If it doesn't, create it
        std::fs::create_dir("./cached").unwrap();
    }

    // Start the server
    rocket::build()
        .mount("/", routes![get_data])
        .mount("/auto/", routes![auto_get_data])
        .register("/", catchers![not_found])
}
