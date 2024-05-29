use log::{error, info};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;

use crate::models::permission::{RolePermissionRequest, Permission, PermissionListResponse, RolePermission, RolePermissionResponse, RoleWithPermissions, Role, RoleResponse};
use crate::responses::response::GenericResponse;


#[get("/role")]
pub async fn get_role(
    pool: &State<PgPool>
) -> Result<Json<RoleResponse>, Status> {
    match sqlx::query_as!(Role, "SELECT id, role_name FROM roles")
        .fetch_all(pool.inner())
        .await
    {
        Ok(roles) => Ok(Json(RoleResponse { role: roles })),
        Err(e) => {
            error!("Error fetching roles: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[get("/permissions")]
pub async fn permission_list(
    pool: &State<PgPool>
) -> Result<Json<PermissionListResponse>, Status> {
    match sqlx::query_as!(
        Permission,
        "SELECT id, permissions_name FROM permissions"
    )
    .fetch_all(pool.inner())
    .await {
        Ok(permissions) => {
            info!("Fetch permission success");
            Ok(Json(PermissionListResponse {
                status: "success".to_string(),
                data: permissions,
            }))
        },
        Err(e) => {
            error!("Permission APi error: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}


#[get("/permission/userRolePermission")]
pub async fn get_role_permission(
    pool: &State<PgPool>
)-> Result<Json<RolePermissionResponse>, Status> {
    let all_roles = sqlx::query!(
        r#"
            SELECT r.id as role_id, r.role_name, p.id as permissions_id, p.permissions_name
            FROM roles r
            LEFT JOIN role_permissions rp ON r.id = rp.role_id
            LEFT JOIN permissions p ON rp.permissions_id = p.id
        "#
    )
    .fetch_all(pool.inner())
    .await;

    match all_roles {
        Ok(records) => {
            let mut role_map: std::collections::HashMap<i32, RoleWithPermissions> = std::collections::HashMap::new();

            for record in records {
                let role_entry = role_map.entry(record.role_id).or_insert_with(|| RoleWithPermissions {
                    id: record.role_id,
                    role_name: record.role_name.clone(),
                    permissions: Vec::new()
                });

                if let Some(permission_id) = record.permissions_id {
                    role_entry.permissions.push(RolePermission {
                        id: permission_id,
                        name: record.permissions_name.clone().unwrap_or_default()
                    });
                }
            }

            let data: Vec<RoleWithPermissions> = role_map.into_iter().map(|(_, role)| {
                let mut unique_permissions = std::collections::HashMap::new();
                for permissions in role.permissions {
                    unique_permissions.entry(permissions.id).or_insert(permissions);
                }

                RoleWithPermissions {
                    id: role.id,
                    role_name: role.role_name,
                    permissions: unique_permissions.into_iter().map(|(_, p)| p).collect(),
                }
            }).collect();

            info!("Fetch role permission success");
            Ok(Json(RolePermissionResponse {
                status: "success".to_string(),
                data
            }))
        },
        Err(e) => {
            error!("Role permission API error: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[post("/permission/addRolePermission", format = "json", data = "<request>")]
pub async fn add_role_permissiom(
    request: Json<RolePermissionRequest>,
    pool: &State<PgPool>
) -> Result<Json<GenericResponse>, Status> {
    let req = request.into_inner();

    let permission = sqlx::query!(
        "SELECT id FROM permissions WHERE permissions_name = $1",
        req.permissions_name
    )
    .fetch_optional(pool.inner())
    .await;

    let permission = match permission {
        Ok(Some(permission)) => permission,
        Ok(None) => {
            error!("Permission not found: {}", req.permissions_name);
            return Ok(Json(GenericResponse {
                status: "error".to_string(),
                message: "add permission api error".to_string()
            }))
        }
        Err(e) => {
            error!("Database error: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    let role = sqlx::query!(
        "SELECT id, role_name FROM roles WHERE id = $1",
        req.role_id
    )
    .fetch_optional(pool.inner())
    .await;

    let role = match role {
        Ok(Some(role)) => role,
        Ok(None) => {
            error!("Role not found:{}", req.role_id);
            return Ok(Json(GenericResponse {
                status: "error".to_string(),
                message: "add permission api error".to_string()
            }))
        }
        Err(e) => {
            error!("Database error: {:?}", e);
            return Err(Status::InsufficientStorage);
        }
    };

    let role_permission = sqlx::query!(
        "SELECT * FROM role_permissions WHERE role_id = $1 AND permissions_id = $2",
        req.role_id, permission.id
    )
    .fetch_optional(pool.inner())
    .await;

    if let Ok(Some(_)) = role_permission {
        error!("Permission is already assined to role: {}", req.role_id);
        return Ok(Json(GenericResponse {
            status: "error".to_string(),
            message: "add permission api error".to_string()
        }))
    }

    let result = sqlx::query!(
        "INSERT INTO role_permissions (role_id, permissions_id) VALUES ($1, $2)",
        req.role_id, permission.id
    )
    .execute(pool.inner())
    .await;

    match result {
        Ok(_) => {
            info!("Added permission to role: {} -> {}", req.role_id, req.permissions_name);
            return Ok(Json(GenericResponse {
                status: "success".to_string(),
                message: "add permission api success".to_string()
            }))
        }
        Err(e) => {
            error!("Database error: {:?}", e);
            Err(Status::InsufficientStorage)
        }
    }    
}

#[post("/permission/deleteRolePermission", format = "json", data = "<request>")]
pub async fn delete_role_permission(
    request: Json<RolePermissionRequest>,
    pool: &State<PgPool>
) -> Result<Json<GenericResponse>, Status> {
    let req = request.into_inner();

    let permission = sqlx::query!(
        "SELECT id FROM permissions WHERE permissions_name = $1",
        req.permissions_name
    )
    .fetch_optional(pool.inner())
    .await;

    let permission = match permission {
        Ok(Some(permission)) => permission,
        Ok(None) => {
            error!("Permission not found:{}", req.permissions_name);
            return Ok(Json(GenericResponse {
                status: "error".to_string(),
                message: "add permission api error".to_string()
            }))
        }
        Err(e) => {
            error!("Database error:{:?}", e);
            return Err(Status::InternalServerError)
        }
    };

    let role = sqlx::query!(
        "SELECT id, role_name FROM roles WHERE id = $1",
        req.role_id
    )
    .fetch_optional(pool.inner())
    .await;

    let role = match role {
        Ok(Some(role)) => role,
        Ok(None) => {
            error!("Role not found:{}", req.role_id);
            return Ok(Json(GenericResponse {
                status: "error".to_string(),
                message: "add permission api error".to_string()
            }))
        }
        Err(e) => {
            error!("Database error:{:?}", e);
            return Err(Status::InternalServerError)
        }
    };

    let role_permission = sqlx::query!(
        "SELECT id FROM role_permissions WHERE role_id = $1 AND permissions_id = $2",
        req.role_id, permission.id
   )
   .fetch_optional(pool.inner())
   .await;

   let role_permission = match role_permission {
        Ok(Some(role_permission)) => role_permission,
        Ok(None) => {
            error!("Role permission not found: role_id = {}, permission_id = {}", req.role_id, permission.id);
            return Ok(Json(GenericResponse {
                status: "error".to_string(),
                message: "add permission api error".to_string()
            }))
        }
        Err(e) => {
            error!("Database error: {:?}", e);
            return Err(Status::InternalServerError);
        }
   };

   let result = sqlx::query!(
        "DELETE FROM role_permissions WHERE id = $1",
        role_permission.id
   )
   .execute(pool.inner())
   .await;

   match result {
        Ok(_) => {
            info!("Deleted permission from role: {} -> {}", req.role_id, req.permissions_name);
            Ok(Json(GenericResponse {
                status: "success".to_string(),
                message: "deleted role permission success".to_string(),
            }))
        }
        Err(e) => {
            error!("Database error: {:?}", e);
            Err(Status::InternalServerError)
        }
   }
}



