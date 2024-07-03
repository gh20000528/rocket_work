use std::result;

use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use sqlx::PgPool;
use rocket::http::Status;
use log::{info, error};
use rocket::serde::Serialize;



use crate::models::worklist::WorklistSettingReq;
use crate::responses::response::GenericResponse;
use crate::tools::dicom::run;


// 定義res, 泛型 T 用於包含不同的數據類型
#[derive(Serialize, Deserialize)]
struct WorklistResponse<T> {
    status: String,
    message: String,
    data: Option<T>,
}

// 定義 DICOM 結構
#[derive(Serialize, Deserialize)]
pub struct DicomData {
    pub accession_number: String,
    pub study_instance_uid: String,
    pub patient_name: String,
    pub patient_id: String,
    pub patient_sex: String,
    pub patient_birth_date: String,
    pub modality: String,
}

// 設定 DICOM addr, called_ae_title, calling_ae_title
#[post("/worklist_setting", format = "json", data = "<worklist_data>")]
pub async fn worklist_setting(
    pool: &State<PgPool>,
    worklist_data: Json<WorklistSettingReq>
) -> Result<Json<GenericResponse>, Status> {
    let data = worklist_data.into_inner();

    // table只放一筆資料, 新的醫院就更新資料
    match sqlx::query!(
        r#"
            UPDATE worklist_setting
            SET port = $1, calling_ae_title = $2, called_ae_title = $3
            WHERE id = 1
        "#,
        data.port,
        data.calling_ae_title,
        data.called_ae_title
    )
    .execute(pool.inner())
    .await {
        Ok(_) => Ok(Json(GenericResponse {
            status: "success".to_string(),
            message: "Worklist setting update success".to_string()
        })),
        Err(e) => {
            error!("Failed to insert worklist setting: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}

// sync worklist 
#[post("/sync_worklist")]
pub async fn sync_worklist(pool: &State<PgPool>) -> Result<Json<WorklistResponse<Vec<DicomData>>>, Status> {
    // 取得設定資料
    let settings = match sqlx::query_as!(
        WorklistSettingReq,
        r#"
            SELECT port, calling_ae_title, called_ae_title
            FROM worklist_setting
            WHERE id = 1
        "#
    )
    .fetch_one(pool.inner())
    .await {
        Ok(settings) => settings,
        Err(e) => {
            error!("Failed to fetch worklist settings: {:?}", e);
            return Err(Status::InternalServerError);
        }
    };

    // 執行 DICOM 
    match run(&settings).await {
        Ok(dicom_data) => Ok(Json(WorklistResponse {
            status: "success".to_string(),
            message: "Worklist synced successfully".to_string(),
            data: Some(dicom_data),
        })),
        Err(e) => {
            error!("Failed to sync worklist: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}


// test
#[cfg(test)]
mod tests {
    use crate::rocket;

    use super::*;
    use rocket::http::hyper::request;
    use rocket::local::asynchronous::Client;
    use rocket::http::{Status, ContentType};
    use sqlx::{PgPool, Executor};
    use dotenv::dotenv;
    use std::env;
    use rocket::serde::json::serde_json;
    use rocket::routes;

    // 設置測試數據庫
    async fn setup_test_db() -> PgPool {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE must be set");
        let pool = PgPool::connect(&database_url).await.unwrap();
        pool
    }

    #[rocket::async_test]
    async fn test_worklist_setting() {
        let pool = setup_test_db().await;
        let rocket = rocket::build()
            .manage(pool.clone())
            .mount("/", routes![worklist_setting]);
        let client = Client::tracked(rocket).await.expect("vaid rocket instance");

        let request_body = WorklistSettingReq {
            port: "127.0.0.1:11112".to_string(),
            calling_ae_title: "EVAS".to_string(),
            called_ae_title: "WORKLIST".to_string(),
        };

        let response = client.post("/worklist_setting")
            .header(ContentType::JSON)
            .body(serde_json::to_string(&request_body).unwrap())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let body: GenericResponse = response.into_json().await.unwrap();
        assert_eq!(body.status, "success");
        assert_eq!(body.message, "Worklist setting update success");
    }

    #[rocket::async_test]
    async fn test_sync_worklist() {
        let pool = setup_test_db().await;
        let rocket = rocket::build()
            .manage(pool)
            .mount("/", routes![sync_worklist]);
        let client = Client::tracked(rocket).await.expect("valid rocket instance");
    
        let response = client.post("/sync_worklist")
            .dispatch()
            .await;
    
        assert_eq!(response.status(), Status::Ok);
    
        let body: WorklistResponse<Vec<DicomData>> = response.into_json().await.unwrap();
        assert_eq!(body.status, "success");
        assert_eq!(body.message, "Worklist synced successfully");
    }
}