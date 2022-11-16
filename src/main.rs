#[macro_use]
extern crate rocket;

use chrono::Datelike;
// import json from rocket
use rocket::fs::NamedFile;
use rocket::tokio::io::AsyncWriteExt;


#[get("/getdata/<day>/<month>")]
async fn get_data(day: u8, month: u8) -> Result<NamedFile, String> {
    if day > 31 || month > 12 {
        return Err("Invalid date".to_string());
    }

    let current_year = chrono::Local::now().year();

    let formatted_date = format!("{}.{}.{}", day, month, current_year);
    let response = match reqwest::get(format!(
        "https://zastepstwa.zschie.pl/pliki/{}.pdf",
        formatted_date
    ))
    .await {
        Ok(response) => response,
        Err(err) => return Err(format!("Error while fetching data: {}", err)),
    };

    // If the server returns a 200 status code
    if response.status() == 200 {
        // Create a new file
        let filename = format!("./cached/{}.pdf", formatted_date);
        let mut file = match rocket::tokio::fs::File::create(&filename).await {
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
        match NamedFile::open(&filename).await {
            Ok(file) => Ok(file),
            Err(err) => Err(format!("Error while opening file: {}", err)),
        }

    } else {
        // Return an error
        let response_status = response.status().as_u16();
        Err(format!("Server returned a {} status code", response_status))
    }
}

#[get("/getdata")]
async fn auto_get_data() -> Result<NamedFile, String> {
    // Get current date
    let current_date = chrono::Local::now();
    // Format the current date to the PL format
    let current_date = current_date.format("%d.%m.%Y").to_string();
    // Send a get request to the server

    let response = match reqwest::get(format!(
        "https://zastepstwa.zschie.pl/pliki/{}.pdf",
        current_date
    ))
    .await {
        Ok(response) => response,
        Err(err) => return Err(format!("Error while fetching data: {}", err)),
    };

    // If the server returns a 200 status code
    if response.status() == 200 {
        // Create a new file
        let filename = format!("./cached/{}.pdf", current_date);
        let mut file = match rocket::tokio::fs::File::create(&filename).await {
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
        match NamedFile::open(&filename).await {
            Ok(file) => Ok(file),
            Err(err) => Err(format!("Error while opening file: {}", err)),
        }

    } else {
        // Return an error
        let response_status = response.status().as_u16();
        Err(format!("Server returned a {} status code", response_status))
    }
}

#[get("/")]
async fn index() -> &'static str {
    "Hello, world! Try to go to /getdata to get the current day's data"
}

#[launch]
fn launch() -> _ {
    rocket::build().mount("/", routes![index, get_data, auto_get_data])
}
