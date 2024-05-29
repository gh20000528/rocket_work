use sqlx::FromRow;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{NaiveDateTime};

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct User {
    id: Option<Uuid>,
    username: String,
    voice_attachment: bool,
    created_at: Option<NaiveDateTime>,
    updated_at: Option<NaiveDateTime>,
    role_id: i32,
    deleted: bool
}

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct UserWithRole {
    pub id: Option<Uuid>,
    pub username: String,
    pub voice_attachment: Option<bool>,
    pub role_id: i32,
    pub deleted: Option<bool>,
    pub role_name: String
}

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct UserInfo {
    pub id: Option<Uuid>,
    pub username: String,
    pub role_id: i32,
}

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct Role {
    pub id: i32,
    pub role_name: String
}
#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct Permission {
    pub id: i32,
    pub permissions_name: String,
}

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub voice_attachment: bool,
    pub role_id: String,
}


#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub captcha: String,
    pub captchaId: String,
}

#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct DeleteUserRequest {
    pub Uid: String,
}


#[derive(Serialize, Deserialize, FromRow, Debug)]
pub struct EditRequest {
    pub Uid: String,
    pub newPassword: String,
}

