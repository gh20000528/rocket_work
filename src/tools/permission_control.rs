use rocket::http::Status;
use rocket::request::{self, FromRequest, Request};
use rocket::outcome::Outcome;
use rocket::State;
use sqlx::PgPool;
use std::collections::HashSet;


use crate::tools::jwt::validate_jwt;

pub struct UserWithPermissions {
    pub user_id: String,
    pub permissions: HashSet<String>
}

#[derive(Debug)]
pub enum PermissionError {
    Unauthorized,
    Forbidden
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserWithPermissions {
    type Error = PermissionError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let pool = request.guard::<&State<PgPool>>().await.unwrap();
        let auth_header = request.headers().get_one("Authorization");

        let token = match auth_header {
            Some(auth_header) => auth_header.split_whitespace().nth(1),
            None => return Outcome::Error((Status::Unauthorized, PermissionError::Unauthorized)),
        };

        let token = match token {
            Some(token) => token,
            None => return Outcome::Error((Status::Unauthorized, PermissionError::Unauthorized))
        };

        let token_data = match validate_jwt(token) {
            Ok(c) => c,
            Err(_) => return Outcome::Error((Status::Unauthorized, PermissionError::Unauthorized))
        };

        let username = token_data.claims.sub;
        let user = sqlx::query!(
            r#"
                SELECT p.permissions_name FROM users u
                JOIN role_permissions rp ON u.role_id = rp.role_id
                JOIN permissions p ON rp.permissions_id = p.id
                WHERE u.username = $1
            "#,
            username
        )
        .fetch_all(pool.inner())
        .await;

        let user = match user{
            Ok(permissions) => permissions,
            Err(_) => return Outcome::Error((Status::Unauthorized, PermissionError::Unauthorized))
        };

        let permissions_set: HashSet<String> = user.into_iter().map(|record| record.permissions_name).collect();

        Outcome::Success(UserWithPermissions {
            user_id: username,
            permissions: permissions_set
        })
    }
}
