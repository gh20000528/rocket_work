use dicom_core::{DataElement, PrimitiveValue, VR, DicomValue, value::DataSetSequence}; // 引入 DICOM 核心模組
use dicom_dictionary_std::tags; // 引入 DICOM 標籤字典
use dicom_dump::DumpOptions; // 引入 DICOM 轉儲選項，用於打印 DICOM 對象
use dicom_encoding::{transfer_syntax, TransferSyntaxIndex}; // 引入 DICOM 編碼和傳輸語法
use dicom_core::header::Length;
use dicom_core::dicom_value; // 引入 DICOM 值模組
use dicom_object::{mem::InMemDicomObject, StandardDataDictionary}; // 引入 DICOM 內存對象和標準數據字典
use dicom_transfer_syntax_registry::{entries, TransferSyntaxRegistry}; // 引入 DICOM 傳輸語法註冊表
use dicom_ul::pdu::Pdu; // 引入 DICOM PDU 模組
use dicom_ul::{
    association::ClientAssociationOptions,
    pdu::{PDataValue, PDataValueType},
}; // 引入 DICOM UL 模組和協會選項
use snafu::prelude::*; // 引入 Snafu 錯誤處理模組
use std::io::{stderr, Read}; // 引入標準輸入輸出模組
use tracing::{debug, error, info, warn, Level};

use crate::controllers::worklist_controller::DicomData;
use crate::models::worklist::WorklistSettingReq; // 引入日誌記錄模組

// 定義可能的錯誤類型
#[derive(Debug, Snafu)]
pub enum Error {
    InitScu {
        source: dicom_ul::association::client::Error,
    },
    CreateCommand { source: dicom_object::ReadError },
    ReadCommand { source: dicom_object::ReadError },
    DumpOutput { source: std::io::Error },
    #[snafu(whatever, display("{}", message))]
    Other {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error + 'static>, Some)))]
        source: Option<Box<dyn std::error::Error + 'static>>,
    },
}


// 執行主要邏輯的函數
pub async fn run(setting: &WorklistSettingReq) -> Result<Vec<DicomData>, Error> {
    // DICOM 伺服器地址和設置
    let addr = setting.port.clone(); // 替換為你的SCP地址
    let calling_ae_title = setting.calling_ae_title.clone(); // 呼叫的 AE 標題
    let called_ae_title = setting.called_ae_title.clone(); // 被呼叫的 AE 標題（替換為 SCP 的 AE 標題）
    let max_pdu_length = 16384; // 最大 PDU 長度
    let verbose = true; // 是否顯示詳細日誌

    // 設置全局日誌記錄
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(if verbose { Level::DEBUG } else { Level::INFO })
            .finish(),
    )
    .unwrap_or_else(|e| {
        error!("{}", snafu::Report::from_error(e));
    });

    // 構建 DICOM 查詢對象
    let dcm_query = build_query(verbose)?;

    // DICOM 抽象語法（例如，模態工作列表信息模型查找）
    let abstract_syntax = dicom_dictionary_std::uids::MODALITY_WORKLIST_INFORMATION_MODEL_FIND;

    if verbose {
        info!("正在與 '{}' 建立連接...", &addr);
    }

    // 設置 SCU（Service Class User）的選項並建立連接
    let scu_opt = ClientAssociationOptions::new()
        .with_abstract_syntax(abstract_syntax)
        .calling_ae_title(calling_ae_title)
        .max_pdu_length(max_pdu_length)
        .called_ae_title(called_ae_title);

    let mut scu = scu_opt.establish_with(&addr).context(InitScuSnafu)?;

    if verbose {
        info!("連接已建立");
    }

    // 選擇第一個 Presentation Context
    let pc_selected = if let Some(pc_selected) = scu.presentation_contexts().first() {
        pc_selected
    } else {
        error!("無法選擇 Presentation Context");
        let _ = scu.abort();
        std::process::exit(-2);
    };
    let pc_selected_id = pc_selected.id;

    // 獲取傳輸語法
    let ts = if let Some(ts) = TransferSyntaxRegistry.get(&pc_selected.transfer_syntax) {
        ts
    } else {
        error!("協商的傳輸語法有誤");
        let _ = scu.abort();
        std::process::exit(-2);
    };

    if verbose {
        debug!("傳輸語法: {}", ts.name());
    }

    // 構建 C-Find 請求命令
    let cmd = find_req_command(abstract_syntax, 1);
    let mut cmd_data = Vec::with_capacity(128);
    cmd.write_dataset_with_ts(&mut cmd_data, &entries::IMPLICIT_VR_LITTLE_ENDIAN.erased())
        .whatever_context("寫入命令失敗")?;

    // 將查詢對象寫入數據集
    let mut iod_data = Vec::with_capacity(128);
    dcm_query
        .write_dataset_with_ts(&mut iod_data, ts)
        .whatever_context("寫入標識符數據集失敗")?;

    let nbytes = cmd_data.len() + iod_data.len();

    if verbose {
        debug!("正在發送查詢 ({} B)...", nbytes);
    }

    // 發送命令 PDU
    let pdu = Pdu::PData {
        data: vec![PDataValue {
            presentation_context_id: pc_selected_id,
            value_type: PDataValueType::Command,
            is_last: true,
            data: cmd_data,
        }],
    };

    scu.send(&pdu).whatever_context("無法發送命令")?;

    // 發送數據 PDU
    let pdu = Pdu::PData {
        data: vec![PDataValue {
            presentation_context_id: pc_selected_id,
            value_type: PDataValueType::Data,
            is_last: true,
            data: iod_data,
        }],
    };
    scu.send(&pdu)
        .whatever_context("無法發送C-Find請求")?;

    if verbose {
        debug!("等待響應...");
    }

    let mut dicom_data_list = Vec::new();
    let mut i = 0;
    loop {
        // 接收響應 PDU
        let rsp_pdu = scu
            .receive()
            .whatever_context("從遠程節點接收響應失敗")?;

        match rsp_pdu {
            Pdu::PData { data } => {
                let data_value = &data[0];
                let cmd_obj = InMemDicomObject::read_dataset_with_ts(
                    &data_value.data[..],
                    &entries::IMPLICIT_VR_LITTLE_ENDIAN.erased(),
                )
                .context(ReadCommandSnafu)?;
                if verbose {
                    eprintln!("匹配 #{} 響應命令:", i);
                    DumpOptions::new()
                        .dump_object_to(stderr(), &cmd_obj)
                        .context(DumpOutputSnafu)?;
                }
                let status = cmd_obj
                    .get(tags::STATUS)
                    .whatever_context("響應中缺少狀態碼")?
                    .to_int::<u16>()
                    .whatever_context("讀取狀態碼失敗")?;
                if status == 0 {
                    if verbose {
                        debug!("匹配完成");
                    }
                    if i == 0 {
                        info!("查詢無匹配結果");
                    }
                    break;
                } else if status == 0xFF00 || status == 0xFF01 {
                    if verbose {
                        debug!("操作待處理: {:x}", status);
                    }

                    let dcm = {
                        let mut rsp = scu.receive_pdata();
                        let mut response_data = Vec::new();
                        rsp.read_to_end(&mut response_data)
                            .whatever_context("讀取響應數據失敗")?;

                        InMemDicomObject::read_dataset_with_ts(&response_data[..], ts)
                            .whatever_context("無法讀取響應數據集")?
                    };

                    println!(
                        "------------------------ 匹配 #{} ------------------------",
                        i
                    );
                    DumpOptions::new()
                        .dump_object(&dcm)
                        .context(DumpOutputSnafu)?;

                    let modality = extract_modality(&dcm);
                    // 将 DICOM 数据转换为 JSON 格式
                    let dicom_data = DicomData {
                        accession_number: dcm.get(tags::ACCESSION_NUMBER).map_or("".to_string(), |v| v.to_str().unwrap_or(std::borrow::Cow::Borrowed("")).to_string()),
                        study_instance_uid: dcm.get(tags::STUDY_INSTANCE_UID).map_or("".to_string(), |v| v.to_str().unwrap_or(std::borrow::Cow::Borrowed("")).to_string()),
                        patient_name: dcm.get(tags::PATIENT_NAME).map_or("".to_string(), |v| v.to_str().unwrap_or(std::borrow::Cow::Borrowed("")).to_string()),
                        patient_id: dcm.get(tags::PATIENT_ID).map_or("".to_string(), |v| v.to_str().unwrap_or(std::borrow::Cow::Borrowed("")).to_string()),
                        patient_sex: dcm.get(tags::PATIENT_SEX).map_or("".to_string(), |v| v.to_str().unwrap_or(std::borrow::Cow::Borrowed("")).to_string()),
                        patient_birth_date: dcm.get(tags::PATIENT_BIRTH_DATE).map_or("".to_string(), |v| v.to_str().unwrap_or(std::borrow::Cow::Borrowed("")).to_string()),
                        modality: modality.expect("REASON"),
                    };

                    dicom_data_list.push(dicom_data);

                    if let Some(status) = dcm.get(tags::STATUS) {
                        let status = status.to_int::<u16>().ok();
                        if status == Some(0) {
                            if verbose {
                                debug!("匹配完成");
                            }
                            break;
                        }
                    }

                    i += 1;
                } else {
                    warn!("操作失敗 (狀態碼 {})", status);
                    break;
                }
            }

            pdu @ Pdu::Unknown { .. }
            | pdu @ Pdu::AssociationRQ { .. }
            | pdu @ Pdu::AssociationAC { .. }
            | pdu @ Pdu::AssociationRJ { .. }
            | pdu @ Pdu::ReleaseRQ
            | pdu @ Pdu::ReleaseRP
            | pdu @ Pdu::AbortRQ { .. } => {
                error!("意外的 SCP 響應: {:?}", pdu);
                let _ = scu.abort();
                std::process::exit(-2);
            }
        }
    }
    let _ = scu.release();

    Ok(dicom_data_list)
}

// 提取嵌套的 MODALITY 数据
fn extract_modality(dicom_obj: &InMemDicomObject<StandardDataDictionary>) -> Option<String> {
    if let Some(data_element) = dicom_obj.element(tags::SCHEDULED_PROCEDURE_STEP_SEQUENCE).ok() {
        if let DicomValue::Sequence(seq) = data_element.value() {
            for item in seq.items() {
                if let Some(nested_element) = item.element(tags::MODALITY).ok() {
                    if let DicomValue::Primitive(PrimitiveValue::Strs(values)) = nested_element.value() {
                        if let Some(modality) = values.first() {
                            return Some(modality.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

// 構建查詢對象，添加查詢的 DICOM 標籤
fn build_query(verbose: bool) -> Result<InMemDicomObject, Error> {
    let mut obj = InMemDicomObject::new_empty();

    // 查詢 Patient Name
    obj.put(DataElement::new(
        tags::PATIENT_NAME,
        VR::PN,
        PrimitiveValue::from("*"),
    ));

    // 添加其他需要查詢的標籤
    obj.put(DataElement::new(
        tags::PATIENT_ID,
        VR::LO,
        PrimitiveValue::from(""),
    ));

    obj.put(DataElement::new(
        tags::STUDY_INSTANCE_UID,
        VR::UI,
        PrimitiveValue::from(""),
    ));

    obj.put(DataElement::new(
        tags::PATIENT_SEX,
        VR::CS,
        PrimitiveValue::from(""),
    ));

    obj.put(DataElement::new(
        tags::PATIENT_BIRTH_DATE,
        VR::DA,
        PrimitiveValue::from(""),
    ));

    obj.put(DataElement::new(
        tags::ACCESSION_NUMBER,
        VR::SH,
        PrimitiveValue::from(""),
    ));

    obj.put(DataElement::new(
        tags::WORKLIST_LABEL,
        VR::LO,
        PrimitiveValue::from(""),
    ));

    // Scheduled Procedure Step Sequence
    let mut sps_sequence = InMemDicomObject::new_empty();

    sps_sequence.put(DataElement::new(
        tags::SCHEDULED_PROCEDURE_STEP_START_DATE,
        VR::DA,
        PrimitiveValue::from(""),
    ));

    sps_sequence.put(DataElement::new(
        tags::MODALITY,
        VR::CS,
        PrimitiveValue::from("ES"),
    ));

    obj.put(DataElement::new(
        tags::SCHEDULED_PROCEDURE_STEP_SEQUENCE,
        VR::SQ,
        DicomValue::Sequence(DataSetSequence::new(vec![sps_sequence], Length::UNDEFINED)),
    ));

    if verbose {
        info!("已構建DICOM查詢對象");
    }
    Ok(obj)
}

// 構建 C-Find 請求命令
fn find_req_command(
    sop_class_uid: &str,
    message_id: u16,
) -> InMemDicomObject<StandardDataDictionary> {
    InMemDicomObject::command_from_element_iter([
        DataElement::new(
            tags::AFFECTED_SOP_CLASS_UID,
            VR::UI,
            PrimitiveValue::from(sop_class_uid),
        ),
        DataElement::new(
            tags::COMMAND_FIELD,
            VR::US,
            dicom_value!(U16, [0x0020]), // C-FIND-RQ
        ),
        DataElement::new(tags::MESSAGE_ID, VR::US, dicom_value!(U16, [message_id])),
        DataElement::new(tags::PRIORITY, VR::US, dicom_value!(U16, [0x0000])), // 中等
        DataElement::new(tags::COMMAND_DATA_SET_TYPE, VR::US, dicom_value!(U16, [0x0001])),
    ])
}
