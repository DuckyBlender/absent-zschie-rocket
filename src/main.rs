use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Info {
    username: String,
}

#[get("/crong")]
async fn index(info: web::Query<Info>) -> Result<String, actix_web::Error> {
    Ok(format!("Welcome {}!", info.username))
}

#[derive(Deserialize)]
struct Date {
    day: u8,
    month: u8,
    year: u16,
}

// /?day=1&month=1&year=2021
#[get("/")]
async fn get_data(info: web::Query<Date>) -> Result<String, actix_web::Error> {
    Ok(format!("The current date is {}.{}.{}", info.day, info.month, info.year))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(get_data)
            .service(index)
            
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}