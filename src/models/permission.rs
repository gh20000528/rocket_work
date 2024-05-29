use sqlx::FromRow;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{NaiveDateTime};


#[derive(Serialize)]
pub struct Permission {
    pub id: i32,
    pub permissions_name: String,
}

#[derive(Serialize)]
pub struct PermissionListResponse {
    pub status: String,
    pub data: Vec<Permission>,
}


#[derive(Serialize)]
pub struct RolePermission {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize)]
pub struct RoleWithPermissions {
    pub id: i32,
    pub role_name: String,
    pub permissions: Vec<RolePermission>,
}

#[derive(Serialize)]
pub struct RolePermissionResponse {
    pub status: String,
    pub data: Vec<RoleWithPermissions>,
}


#[derive(Deserialize)]
pub struct RolePermissionRequest {
    pub role_id: i32,
    pub permissions_name: String,
}

