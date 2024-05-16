use crate::models::todo::Todo;
use crate::models::user::User;
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct GenericResponse {
    pub status: String,
    pub message: String,
}

#[derive(Serialize, Debug)]
pub struct TodoDate {
    pub todo: Todo
}

#[derive(Serialize, Debug)]
pub struct SingleTodoResponse {
    pub status: String,
    pub data: TodoDate
}

#[derive(Serialize, Debug)]
pub struct TodoListResponse {
    pub status: String,
    pub results: usize,
    pub todos: Vec<Todo>
}

#[derive(Serialize, Debug)]
pub struct UserListResponse {
    pub status: String,
    pub results: usize,
    pub users: Vec<User>
}

#[derive(Serialize, Debug)]
pub struct CaptchaResponse {
    pub captcha_image: String,
    pub captcha_id: String,
    pub expires_in: i64,  // 秒数
}