use sqlx::FromRow;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{NaiveDateTime};

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct User {
    id: Option<Uuid>,
    username: String,
    password: String,
    voice_attachment: bool,
    created_at: Option<NaiveDateTime>,
    updated_at: Option<NaiveDateTime>,
    role_id: i32,
}


#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub voice_attachment: bool,
    pub role_id: i32,
}

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub captcha_id: String,
    pub captcha_value: String
}
