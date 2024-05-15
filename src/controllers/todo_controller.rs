use crate::models::todo::{AppState, Todo, UpdateTodoSchema};
use crate::responses::todo_response::{TodoListResponse, TodoDate, GenericResponse, SingleTodoResponse};
use chrono::prelude::*;
use rocket::{
    delete, get, http::Status, patch, post, response::status::Custom, serde::json::Json, State,
};
use uuid::Uuid;

// 使用 Rocket 框架的 get 宏來標記這個函數處理 GET 請求到 "/todo" 路徑，並接收可選的 `page` 和 `limit` 查詢參數。
#[get("/todo?<page>&<limit>")]
pub async fn todos_list_handler(
    page: Option<usize>,       // 可選參數 `page` 表示客戶想查看的頁數，使用 Option 以允許空值（無提供時）。
    limit: Option<usize>,      // 可選參數 `limit` 表示每頁顯示的項目數量，同樣使用 Option。
    data: &State<AppState>    // 從 Rocket 狀態中獲取應用程序狀態，其中包含待辦事項的數據庫（Todo數據庫）。
) -> Result<Json<TodoListResponse>, Status> { // 函數返回一個 JSON 響應或一個 HTTP 狀態。
    let ves = data.todo_db.lock().unwrap();    // 鎖定 todo_db 以獲取對數據的獨佔訪問。使用 `unwrap()` 處理任何潛在的錯誤。

    let limit = limit.unwrap_or(10);           // 如果未提供 `limit`，則默認為每頁10條記錄。
    let offset = (page.unwrap_or(1) - 1) * limit; // 計算需要跳過的記錄數，從而實現分頁。

    let todos: Vec<Todo> = ves.clone().into_iter() // 從數據庫中複製數據，並轉換成迭代器。
                               .skip(offset)       // 跳過前面計算的 `offset` 條記錄。
                               .take(limit)        // 從剩下的記錄中取出 `limit` 指定數量的記錄。
                               .collect();         // 將迭代器中的項目收集到 Vec 中。

    let json_response = TodoListResponse {
        status: "success".to_string(),    // 設置響應狀態為 "success"。
        results: todos.len(),             // 返回實際返回的待辦事項數量。
        todos,                            // 將收集到的待辦事項列表嵌入到響應中。
    };
    Ok(Json(json_response))              // 返回包含待辦事項列表的 JSON 響應。
}

#[post("/todos", data = "<body>")]
pub async fn create_todo_handler(
    mut body: Json<Todo>,
    data: &State<AppState>,
) -> Result<Json<SingleTodoResponse>, Custom<Json<GenericResponse>>> {
    let mut vec = data.todo_db.lock().unwrap();

    for todo in vec.iter() {
        if todo.title == body.title {
            let error_response = GenericResponse{
                status: "fail".to_string(),
                message: format!("Todo with title : '{}' already exists", body.title)
            };
            return Err(Custom(Status::Conflict, Json(error_response)))
        }
    }

    let uuid_id = Uuid::new_v4();
    let DateTime = Utc::now();

    body.id = Some(uuid_id.to_string());
    body.completed = Some(false);
    body.createdAt = Some(DateTime);
    body.updatedAt = Some(DateTime);

    let todo = body.to_owned();
    
    vec.push(body.into_inner());

    let json_response = SingleTodoResponse {
        status: "success".to_string(),
        data: TodoDate {
            todo: todo.into_inner()
        }
    };
    Ok(Json(json_response))
}

#[get("/todos/<id>")]
pub async fn get_todo_handler(
    id: String,
    data: &State<AppState>,
) -> Result<Json<SingleTodoResponse>, Custom<Json<GenericResponse>>> {
    let vec = data.todo_db.lock().unwrap();

    for todo in vec.iter() {
        if todo.id == Some(id.to_owned()) {
            let json_response = SingleTodoResponse {
                status: "success".to_string(),
                data: TodoDate { todo: todo.clone() }
            };
            return Ok(Json(json_response))
        }
    }

    let error_response = GenericResponse {
        status: "fail".to_string(),
        message: format!("Todo with Id: {} not found", id)
    };
    Err(Custom(Status::NotFound, Json(error_response)))
}

#[patch("/todo/<id>", data = "<body>")]
pub async fn edit_todo_handler(
    id: String,
    body: Json<UpdateTodoSchema>,
    data: &State<AppState>
) -> Result<Json<SingleTodoResponse>, Custom<Json<GenericResponse>>> {
    let mut vec = data.todo_db.lock().unwrap();

    for todo in vec.iter_mut() {
        if todo.id == Some(id.clone()) {
            let datetime = Utc::now();
            let title = body.title.to_owned().unwrap_or(todo.title.to_owned());
            let content = body.content.to_owned().unwrap_or(todo.content.to_owned());
            let payload = Todo {
                id: todo.id.to_owned(),
                title: if !title.is_empty() {
                    title
                } else {
                    todo.title.to_owned()
                },
                content: if !content.is_empty() {
                    content
                } else {
                    todo.content.to_owned()
                },
                completed: if body.completed.is_some() {
                    body.completed
                } else {
                    todo.completed
                },
                createdAt: todo.createdAt,
                updatedAt: Some(datetime)
            };
            *todo = payload;

            let json_response = SingleTodoResponse {
                status: "success".to_string(),
                data: TodoDate { todo: todo.clone() }
            };

            return Ok(Json(json_response))
        }
    }

    let error_response = GenericResponse {
        status: "fail".to_string(),
        message: format!("Todo with ID: {} not found", id)
    };
    Err(Custom(Status::NotFound, Json(error_response)))
}

#[delete("/todos/<id>")]
pub async fn delete_todo_handler(
    id: String,
    data: &State<AppState>
) -> Result<Status, Custom<Json<GenericResponse>>> {
    let mut vec = data.todo_db.lock().unwrap();

    for todo in vec.iter_mut() {
        if  todo.id == Some(id.clone()) {
            vec.retain(|todo| todo.id != Some(id.to_owned()));
            return Ok(Status::NoContent)
        }
    }

    let error_response = GenericResponse {
        status: "fail".to_string(),
        message: format!("Todo with ID: {} not found", id)
    };
    Err(Custom(Status::NotFound, Json(error_response)))
}