use sqlx::FromRow;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDateTime};

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