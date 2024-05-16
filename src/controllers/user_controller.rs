use std::time::SystemTime;

use rocket::http::{Cookie, CookieJar, Status};
use rocket::{State};
use rocket::serde::json::Json;
use sqlx::PgPool;
use log::{info, warn, error};


use crate::models::user::{LoginRequest, RegisterRequest, User};
use crate::responses::todo_response::{UserListResponse, CaptchaResponse};
use crate::GenericResponse;
use crate::models::captcha::{CaptchaStore, generate_captcha};
use crate::tools::jwt::{generate_jwt};

// tool function
async fn validate_captcha(captcha_id: &str, captcha_value: &str, store: &CaptchaStore) -> Result<(), Status> {
    let mut store = store.lock().expect("Faild to lock store");
    if let Some(captcha_info) = store.get(captcha_id) {
        if captcha_info.expires < SystemTime::now() || captcha_info.captcha != captcha_value {
            store.remove(captcha_id);
            Err(Status::BadRequest)
        } else {
            store.remove(captcha_id);
            Ok(())
        }
    } else {
        Err(Status::BadRequest)
    }
}


// api
#[get("/users")]
pub async fn get_users(pool: &State<PgPool>) -> Result<Json<UserListResponse>, Status> {
    let users: Vec<User> = match sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(pool.inner())
        .await {
            Ok(users) => users,
            Err(e) => {
                println!("{}", e);
                return Err(Status::InternalServerError)
            }
        };

    Ok(Json(UserListResponse {
        status: "success".to_string(),
        results: users.len(),
        users
    }))
}

// register
#[post("/register", format = "json", data = "<register_data>")]
pub async fn register(register_data: Json<RegisterRequest>, pool: &State<PgPool>) -> Result<Json<GenericResponse>, Status> {
    let reg_data = register_data.into_inner();

    if reg_data.password.len() < 8 {
        return  Err(Status::BadRequest);
    }

    let hashed_password = match bcrypt::hash(&reg_data.password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return Err(Status::InternalServerError)
    };

    match sqlx::query!(
        "INSERT INTO users (username, password, voice_attachment, role_id) VALUES($1, $2, $3, $4) RETURNING id",
        reg_data.username, hashed_password, reg_data.voice_attachment, reg_data.role_id
    )
    .fetch_one(pool.inner())
    .await {
        Ok(user) => Ok(Json(GenericResponse { status: "success".to_string(), message: "User resister success".to_string() })),
        Err(_) => Err(Status::InternalServerError)
    }
}

// captcha
#[get("/captcha")]
pub async fn generate_captcha_handler(store: &State<CaptchaStore>) -> Result<Json<CaptchaResponse>, Status> {
    let (captcha_id, captcha_image) = generate_captcha(store.inner()).await;

    let captcha_response = CaptchaResponse {
        captcha_image,
        captcha_id,
        expires_in: 60
    };

    Ok(Json(captcha_response))
}

// login
#[post("/login", format = "json", data = "<login_data>")]
pub async fn login(
    login_data: Json<LoginRequest>,
    pool: &State<PgPool>,
    captcha_store: &State<CaptchaStore>,
    cookies: &CookieJar<'_>
) -> Result<Json<GenericResponse>, Status> {
    let login = login_data.into_inner();
    info!("Attempting to login user: {}", login.username);
    // vaild captcha
    validate_captcha(&login.captcha_id, &login.captcha_value, captcha_store.inner()).await;

    // user and password validation
    let result = sqlx::query!(
        "SELECT * FROM users WHERE username = $1",
        login.username,
    )
    .fetch_optional(pool.inner())
    .await;


    match result {
        Ok(Some(user)) => {
            if bcrypt::verify(&login.password, &user.password).unwrap_or(false) {
                match generate_jwt(&login.username).await{
                    Ok(token) => {
                        println!("Token : {}", token);
                        cookies.add(
                            Cookie::build(("user_token", token))
                                .path("/")
                                .secure(true)
                                .http_only(true)
                                .finish(),
                        );
                        info!("User {} logged in successfully", login.username);
                        Ok(Json(GenericResponse { status: "success".to_string(), message: "login success".to_string() }))
                    }
                    Err(e) => {
                        error!("Failed to generate JWT: {:?}", e);
                        Err(Status::InternalServerError)
                    }
                }
            } else {
                warn!("Invalid password attempt for user {}", login.username);
                Err(Status::Unauthorized)
            }
        },
        Ok(None) => {
            warn!("No user found with username: {}", login.username);
            Err(Status::Unauthorized)
        },
        Err(e) => {
            error!("Database error occurred: {:?}", e);
            Err(Status::InternalServerError)
        }
    }

}