use std::collections::HashSet;
use std::time::SystemTime;

use regex::Regex;
use rocket::futures::lock::Mutex;
use rocket::http::{Cookie, CookieJar, HeaderMap, SameSite, Status};
use rocket::request::{self, FromRequest, Request};
use rocket::serde::json::Json;
use rocket::outcome::Outcome; 
use rocket::State;
use sqlx::PgPool;
use log::{info, warn, error};


use crate::models::user::{DeleteUserRequest, LoginRequest, Permission, RegisterRequest, Role, UserWithRole, UserInfo, EditRequest };
use crate::responses::response::{UserListResponse, CaptchaResponse, UserInfoResponse, GenericResponse, LoginResponse};
use crate::models::captcha::{CaptchaStore, generate_captcha};
use crate::tools::jwt::{generate_jwt, validate_jwt};
use crate::tools::permission_control::UserWithPermissions;


// init token black
pub struct TokenBlack {
    black: Mutex<HashSet<String>>
}

impl TokenBlack {
    pub fn new() -> Self {
        TokenBlack {
            black: Mutex::new(HashSet::new())
        }
    }

    async fn add(&self, token: String) {
        let mut blacklist = self.black.lock().await;
        blacklist.insert(token);
    }
}

struct RequestHeaders<'h>(&'h HeaderMap<'h>);

#[derive(Debug)]
enum ApiTokenError {
    Missing,
    Invalid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestHeaders<'r> {
    type Error = ApiTokenError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let request_headers = request.headers();
        Outcome::Success(RequestHeaders(request_headers))
    }
}


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

fn valided_password(password: &str) -> bool {
    let re = Regex::new(r"^[A-Za-z\d]{8,}$").unwrap();
    re.is_match(password) && password.chars().any(|c| c.is_alphabetic()) && password.chars().any(|c| c.is_numeric())
}


// api
// user list
#[get("/user")]
pub async fn get_users(pool: &State<PgPool>) -> Result<Json<UserListResponse>, Status> {
    let user: Vec<UserWithRole> = match sqlx::query_as!(
        UserWithRole,
        r#"
            SELECT users.id, users.username, 
            users.voice_attachment, users.role_id, users.deleted, roles.role_name
            FROM users
            JOIN roles ON users.role_id = roles.id
        "#
    )
    .fetch_all(pool.inner())
    .await {
        Ok(users) => users,
        Err(e) => {
            error!("get user list error");
            return Err(Status::InternalServerError);
        }
    };

    Ok(Json(UserListResponse {
        status: "success".to_string(),
        data: user
    }))
}

// register
#[post("/user/register", format = "json", data = "<register_data>")]
pub async fn register(
    register_data: Json<RegisterRequest>,
    pool: &State<PgPool>,
    user_with_permissions: UserWithPermissions
) -> Result<Json<GenericResponse>, Status> {
    let reg_data = register_data.into_inner();

    if !user_with_permissions.permissions.contains("newUser") {
        return  Err(Status::Unauthorized);
    }

    if !valided_password(&reg_data.password) {
        return Err(Status::BadRequest);
    }

    let hashed_password = match bcrypt::hash(&reg_data.password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return Err(Status::InternalServerError)
    };

    let role_id = match reg_data.role_id.parse::<i32>() {
        Ok(id) => id,
        Err(_) => return Err(Status::BadRequest),  // 转换失败时返回 400 Bad Request
    };

    match sqlx::query!(
        "INSERT INTO users (username, password, voice_attachment, role_id) VALUES($1, $2, $3, $4) RETURNING id",
        reg_data.username, hashed_password, reg_data.voice_attachment, role_id
    )
    .fetch_one(pool.inner())
    .await {
        Ok(user) => Ok(Json(GenericResponse { status: "success".to_string(), message: "User resister success".to_string() })),
        Err(_) => Err(Status::InternalServerError)
    }
}

// captcha
#[get("/user/captcha")]
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
#[post("/user/login", format = "json", data = "<login_data>")]
pub async fn login(
    login_data: Json<LoginRequest>,
    pool: &State<PgPool>,
    captcha_store: &State<CaptchaStore>,
    cookies: &CookieJar<'_>
) -> Result<Json<LoginResponse>, Status> {
    let login = login_data.into_inner();
    info!("Attempting to login user: {}", login.username);
    // vaild captcha
    validate_captcha(&login.captchaId, &login.captcha, captcha_store.inner()).await;

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
                            Cookie::build(("user_token", token.clone()))
                                .path("/")
                                .secure(true)
                                .http_only(true)
                                .same_site(SameSite::Lax)
                                .build()
                        );
                        info!("User {} logged in successfully", login.username);
                        Ok(Json(LoginResponse { status: "success".to_string(), message: "login success".to_string(), token: Some(token) }))
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

// logout 
// Implement the logout handler
#[post("/user/logout")]
pub async fn logout(
    token_black: &State<TokenBlack>,
    headers: RequestHeaders<'_>
) -> Result<Json<GenericResponse>, Status> {
    let RequestHeaders(header_map) = headers;
    let auth_header = header_map.get_one("Authorization");

    match auth_header {
        Some(auth_header) => {
            // 提取標頭中的token部分（假設格式為"Bearer <token>"）
            let token = auth_header.split_whitespace().nth(1);
            match token {
                Some(t) => {
                    token_black.add(t.to_string()).await;
                    info!("Logout successful");
                    Ok(Json(GenericResponse { status: "success".to_string(), message: "Logged out successfully".to_string() }))
                },
                None => {
                    error!("Logout API error: No token provided");
                    Err(Status::BadRequest)
                }
            }
        },
        None => {
            error!("Logout API error: No Authorization header provided");
            Err(Status::BadRequest)
        }
    }
}


// user info 
#[get("/user/userinfo")]
pub async fn get_userinfo(
    headers: RequestHeaders<'_>,
    pool: &State<PgPool>,
    token_black: &State<TokenBlack>
) -> Result<Json<UserInfoResponse>, Status> {
    let RequestHeaders(header_map) = headers;
    let auth_header = header_map.get_one("Authorization");

    let token = match auth_header {
        Some(auth_header) => auth_header.split_whitespace().nth(1),
        None => return Err(Status::Unauthorized)
    };

    let token = match token {
        Some(token) => token,
        None => return Err(Status::Unauthorized)
    };

    if token_black.black.lock().await.contains(token) {
        return Err(Status::Unauthorized);
    }

    let claims = match validate_jwt(token) {
        Ok(claims) => claims,
        Err(_) => return  Err(Status::Unauthorized)
    };

    let username = claims.claims.sub;

    let user: Option<UserInfo> = sqlx::query_as!(
        UserInfo,
        "SELECT id, username, role_id FROM users WHERE username = $1",
        username
    )
    .fetch_optional(pool.inner())
    .await
    .unwrap();

    let user = match user {
        Some(user) => user,
        None => return  Err(Status::NotFound)
    };

    let role: Option<Role> = sqlx::query_as!(
        Role,
        "SELECT id, role_name FROM roles WHERE id = $1",
        user.role_id
    )
    .fetch_optional(pool.inner())
    .await
    .unwrap();

    let role = match role {
        Some(role) => role,
        None => return Err(Status::NotFound) 
    };

    let permission: Vec<Permission> = sqlx::query_as!(
        Permission,
        "
            SELECT permissions.id, permissions_name
            FROM permissions
            JOIN role_permissions ON permissions.id = role_permissions.permissions_id
            WHERE role_permissions.role_id = $1
        ",
        user.role_id
    )
    .fetch_all(pool.inner())
    .await
    .unwrap();

    let username_clone= user.username.clone();
    let permission_list: Vec<String> = permission.into_iter().map(|p| p.permissions_name).collect();

    let user_info = UserInfoResponse {
        username: user.username,
        role: role.role_name,
        permissions: permission_list
    };

    info!("Fetch user info for username: {}", username_clone);
    Ok(Json(user_info))

}

// delete user
#[post("/user/softDeleted", format = "json", data = "<delete_data>")]
pub async fn soft_delete_user(
    delete_data: Json<DeleteUserRequest>,
    pool: &State<PgPool>,
    user_with_permissions: UserWithPermissions
) -> Result<Json<GenericResponse>, Status> {
    let delete_request = delete_data.into_inner();

    if !user_with_permissions.permissions.contains("deletedUser") {
        return Err(Status::Forbidden);
    }

    let uuid = match uuid::Uuid::parse_str(&delete_request.Uid) {
        Ok(u) => u,
        Err(_) => return Err(Status::BadRequest)
    };

    match sqlx::query!(
        "UPDATE users SET deleted = TRUE WHERE id = $1",
        uuid
    )
    .execute(pool.inner())
    .await {
        Ok(_) => {
            info!("Soft deleted user id :{}", delete_request.Uid);
            Ok(Json(GenericResponse { status: "success".to_string(), message: "User soft deleted success".to_string() }))
        },
        Err(e) => {
            error!("soft delete api error: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

// edit user
#[post("/user/editpassword", format = "json", data = "<edit_data>")]
pub async fn edit_password(
    edit_data: Json<EditRequest>,
    pool: &State<PgPool>,
    user_with_permissions: UserWithPermissions
) -> Result<Json<GenericResponse>, Status> {
    let edit_req = edit_data.into_inner();

    if !user_with_permissions.permissions.contains("editPassword") {
        return Err(Status::Forbidden);
    }

    let uuid = match uuid::Uuid::parse_str(&edit_req.Uid) {
        Ok(u) => u,
        Err(_) => return Err(Status::BadRequest)
    };

    if !valided_password(&edit_req.newPassword) {
        return Err(Status::BadRequest);
    }

    let hashed_password = match bcrypt::hash(&edit_req.newPassword, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return Err(Status::InternalServerError)
    };

    match sqlx::query!(
        "UPDATE users SET password = $1 WHERE id = $2",
        hashed_password,
        uuid
    )
    .execute(pool.inner())
    .await {
        Ok(_) => {
            info!("Updated password for user");
            Ok(Json(GenericResponse { status: "success".to_string(), message: "password update succedd".to_string() }))
        },
        Err(e) => {
            error!("Error updateing password: {}", e);
            Err(Status::InsufficientStorage)
        }
    }
}