#[macro_use]
extern  crate rocket;

// import rocket 
use std::sync::Mutex;
use std::collections::HashMap;
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions};
use dotenv::dotenv;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

use crate::controllers::user_controller::{ get_users, register, generate_captcha_handler, login, logout, TokenBlack, get_userinfo, soft_delete_user, edit_password };
use crate::controllers::permission_controller::{ permission_list, get_role_permission, add_role_permissiom, delete_role_permission, get_role };
use crate::controllers::worklist_controller::{worklist_setting, sync_worklist};
use crate::models::captcha::CaptchaInfo;

mod db;
mod responses;
mod models;
mod controllers;
mod tools;



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
    .manage(TokenBlack::new())
    .manage(Mutex::new(HashMap::<String, CaptchaInfo>::new()))
    .mount("/", Scalar::with_url("/apidoc", tools::apidoc::ApiDoc::openapi()))
    .mount(
        "/api", 
        routes![
            get_users,
            register,
            generate_captcha_handler,
            login,
            logout,
            get_userinfo,
            soft_delete_user,
            edit_password,
            permission_list,
            get_role_permission,
            add_role_permissiom, 
            delete_role_permission,
            get_role,
            worklist_setting,
            sync_worklist,
        ]
    )
}


#[cfg(test)]
mod tests;