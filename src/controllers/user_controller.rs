use rocket::http::Status;
use rocket::State;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::models::user::User;
use crate::responses::todo_response::UserListResponse;

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