use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct WorklistSettingReq {
    pub port: String,
    pub calling_ae_title: String,
    pub called_ae_title: String
}

