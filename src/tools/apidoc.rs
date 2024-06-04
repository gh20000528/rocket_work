use crate::controllers::{permission_controller, user_controller};
use crate::models::permission::{Permission, Role, RolePermission, RoleResponse};
use crate::models::user::{User, UserInfo};
use crate::responses::response::{GenericResponse, UserInfoResponse, UserListResponse};

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        user_controller::get_users,
        user_controller::get_userinfo,
        user_controller::soft_delete_user,
        user_controller::edit_password,
        user_controller::register,
        user_controller::generate_captcha_handler,
        user_controller::login,
        user_controller::logout,
        permission_controller::permission_list,
        permission_controller::get_role_permission,
        permission_controller::add_role_permissiom,
        permission_controller::delete_role_permission,
        permission_controller::get_role
    ),
    components(
        schemas(User, UserInfo, Permission, RolePermission, Role),
        responses(UserListResponse,UserInfoResponse,GenericResponse, RoleResponse),
    ),
    // tags(
    //     (name = "user::api", description = "User management endpoints."),
    //     (name = "permission::api", description = "Permission management endpoints."),
    // ),
    // modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

// struct SecurityAddon;

// impl Modify for SecurityAddon {
//     fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
//         let components = openapi.components.as_mut().unwrap(); // we can unwrap safely since there already is components registered.
//         components.add_security_scheme(
//             "api_key",
//             SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("todo_apikey"))),
//         )
//     }
// }
