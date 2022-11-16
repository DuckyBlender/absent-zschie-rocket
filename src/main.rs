#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
use rocket_contrib::json::Json;
use serde::{Serialize, Deserialize};
use reqwest::blocking::Client;

#[derive(Debug, Serialize)]
struct StatusMessage {
    status: u16
}

#[get("/get/<day>/<month>")]
fn get(day: u8, month: u8) -> Json<StatusMessage> {
    // Get current date
    let current_date = chrono::Local::now();
    // Format the current date to the PL format
    let current_date = current_date.format("%d.%m.%Y").to_string();
    // Send a get request to the server

    let response = reqwest::blocking::get(format!("https://zastepstwa.zschie.pl/pliki/{}.pdf", current_date));
    
    // If the server returns a 200 status code
    if response.status() == 200 {
        return Json(StatusMessage { status: 200 });
    } else {
        // Return an error
        let response_status: u16 = response.status().as_u16();
        return Json(StatusMessage { status: response_status });
    }
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

fn main() {
    rocket::ignite().mount("/", routes![index, get]).launch();
}
