use std::collections::HashMap;
use std::sync::Mutex;
use uuid:: Uuid;
use std::time::{SystemTime, Duration};
use captcha::{Captcha};

pub struct CaptchaInfo {
    pub captcha: String,
    pub expires: SystemTime,
}

pub type CaptchaStore = Mutex<HashMap<String, CaptchaInfo>>;

pub async fn generate_captcha(store: &CaptchaStore) -> (String, String) {
    let mut captcha = Captcha::new();

    captcha
        .add_chars(4)
        .apply_filter(captcha::filters::Noise::new(0.1))
        .view(220, 100);

    let captcha_id = Uuid::new_v4().to_string();
    let captcha_text = captcha.chars_as_string();

    let mut store = store.lock().expect("Captcha store lock");
    store.insert(captcha_id.clone(), CaptchaInfo {
        captcha: captcha_text.clone(),
        expires: SystemTime::now() + Duration::new(60, 0)
    });

    (captcha_id, captcha.as_base64().unwrap())
}