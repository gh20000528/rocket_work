use crate::models::user::UserWithRole;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct GenericResponse {
    pub status: String,
    pub message: String,
}


#[derive(Serialize, Debug)]
pub struct UserListResponse {
    pub status: String,
    pub data: Vec<UserWithRole>
}

#[derive(Serialize, Debug)]
pub struct CaptchaResponse {
    pub captcha_image: String,
    pub captcha_id: String,
    pub expires_in: i64,  // 秒数
}



#[derive(Serialize)]
pub struct LogoutResponse {
    pub status: String,
}

#[derive(Serialize)]
pub struct UserInfoResponse {
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>
}


#[derive(Serialize)]
pub struct LoginResponse {
    pub status: String,
    pub message: String,
    pub token: Option<String>, // Add token to response
}
