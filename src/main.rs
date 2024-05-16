
// import rocket 
use rocket::{get, http::Status, serde::json::Json};
use serde::Serialize;
use std::sync::Mutex;
use std::collections::HashMap;
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use dotenv::dotenv;

// use crate::controllers::todo_controller::{create_todo_handler, delete_todo_handler, edit_todo_handler, todos_list_handler, get_todo_handler};
use crate::controllers::user_controller::{ get_users, register, generate_captcha_handler, login };
use crate::models::captcha::CaptchaInfo;

mod db;
mod responses;
mod models;
mod controllers;
mod tools;

#[macro_use]
extern  crate rocket;

#[derive(Serialize)]
pub struct GenericResponse {
    pub status: String,
    pub message: String,
}

#[get("/healthchecker")]
pub async fn health_checker_handler() -> Result<Json<GenericResponse>, Status> {
    const MESSAGE: &str = "Build Simple CRUD API with rust and Rocket";

    let response_json = GenericResponse {
        status: "success".to_string(),
        message: MESSAGE.to_string(),
    };
    Ok(Json(response_json))
}

#[launch]
async fn rocket() -> _ {
    dotenv().ok();
    env_logger::init();
    let db_pool = db::init_db().await;

    if let Err(e) = db::test_db_connection(&db_pool).await {
        eprintln!("Failed to connect to the database: {:?}", e);
        std::process::exit(1);
    }

    let cors = CorsOptions {
        allowed_origins: AllowedOrigins::all(),
        allowed_methods: vec![rocket::http::Method::Get, rocket::http::Method::Post, rocket::http::Method::Options]
        .into_iter()
        .map(From::from)
        .collect(),
        allowed_headers: AllowedHeaders::some(&[
            "Authorization",
            "Accept",
            "Content-Type",
        ]),
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors().unwrap();

    rocket::build()
    .attach(cors)
    .manage(db_pool)
    .manage(Mutex::new(HashMap::<String, CaptchaInfo>::new()))
    .mount(
        "/api", 
        routes![
            get_users,
            register,
            generate_captcha_handler,
            login
        ]
    )
}