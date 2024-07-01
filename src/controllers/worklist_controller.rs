use std::result;

use rocket::serde::json::Json;
use rocket::State;
use sqlx::PgPool;
use rocket::http::Status;
use log::{info, error};
use rocket::serde::Serialize;



use crate::models::worklist::WorklistSettingReq;
use crate::responses::response::GenericResponse;
use crate::tools::dicom::run;


#[derive(Serialize)]
struct WorklistResponse<T> {
    status: String,
    message: String,
    data: Option<T>,
}

#[derive(Serialize)]
pub struct DicomData {
    pub accession_number: String,
    pub study_instance_uid: String,
    pub patient_name: String,
    pub patient_id: String,
    pub patient_sex: String,
    pub patient_birth_date: String,
    pub modality: String,
}

#[post("/worklist_setting", format = "json", data = "<worklist_data>")]
pub async fn worklist_setting(
    pool: &State<PgPool>,
    worklist_data: Json<WorklistSettingReq>
) -> Result<Json<GenericResponse>, Status> {
    let data = worklist_data.into_inner();

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


#[post("/sync_worklist")]
pub async fn sync_worklist(pool: &State<PgPool>) -> Result<Json<WorklistResponse<Vec<DicomData>>>, Status> {
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