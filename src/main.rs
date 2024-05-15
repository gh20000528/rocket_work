use models::todo::AppState;
// import rocket 
use rocket::{get, http::Status, response, serde::json::Json};
use serde::Serialize;

// use crate::controllers::todo_controller::{create_todo_handler, delete_todo_handler, edit_todo_handler, todos_list_handler, get_todo_handler};
use crate::controllers::user_controller::{ get_users };

mod db;
mod responses;
mod models;
mod controllers;

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
    let db_pool = db::init_db().await;

    if let Err(e) = db::test_db_connection(&db_pool).await {
        eprintln!("Failed to connect to the database: {:?}", e);
        std::process::exit(1);
    }
    rocket::build()
    .manage(db_pool)
    .mount(
        "/api", 
        routes![
            get_users
        ]
    )
}