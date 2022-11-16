#[macro_use]
extern crate rocket;

// import json from rocket
use rocket::fs::NamedFile;
use rocket::tokio::io::AsyncWriteExt;
use tokio::stream;

#[get("/get/<day>/<month>")]
async fn get(day: u8, month: u8) -> Result<NamedFile, String> {
    // Get current date
    let current_date = chrono::Local::now();
    // Format the current date to the PL format
    let current_date = current_date.format("%d.%m.%Y").to_string();
    // Send a get request to the server

    let response = reqwest::get(format!(
        "https://zastepstwa.zschie.pl/pliki/{}.pdf",
        current_date
    ))
    .await
    .unwrap();

    // If the server returns a 200 status code
    if response.status() == 200 {
        // Create a new file
        let filename = format!("./cached/{}.pdf", current_date);
        let mut file = rocket::tokio::fs::File::create(&filename).await.unwrap();
        // Download the PDF
        let filebytes = response.bytes().await.unwrap();
        // Write the PDF to the file
        file.write_all(&filebytes).await.unwrap();
        // Return the file
        Ok(NamedFile::open(&filename).await.unwrap())

    } else {
        // Return an error
        let response_status: u16 = response.status().as_u16();
        Err(format!("Server returned a {} status code", response_status))
    }
}

#[get("/")]
async fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn launch() -> _ {
    rocket::build().mount("/", routes![index, get])
}
