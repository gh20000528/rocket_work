#[cfg(test)]
mod tests {
    use super::*;
    use rocket::local::asynchronous::Client;
    use rocket::http::{Status, ContentType};
    use sqlx::{PgPool, Executor};
    use dotenv::dotenv;
    use std::collections::HashMap;
    use std::env;
    use rocket::serde::json::serde_json;
    use rocket::routes;
    
    use crate::controllers::user_controller::{login, register, generate_captcha_handler};
    use crate::models::captcha::CaptchaStore;

    async fn setup_test_db() -> PgPool {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&database_url).await.unwrap();

        pool
    }
    
    #[rocket::async_test]
    async fn test_login() {
        let db = setup_test_db().await;
        let captcha_stroe = CaptchaStore::new(HashMap::new());

        // 启动 Rocket 实例
        let rocket = rocket::build()
            .manage(db) // 管理数据库连接池
            .manage(captcha_stroe) // 管理验证码状态
            .mount("/", routes![register, login, generate_captcha_handler]); // 挂载路由

        let client = Client::tracked(rocket).await.expect("valid rocket instance");

        // 生成验证码
        let captcha_response = client.get("/user/captcha")
            .dispatch()
            .await;
        assert_eq!(captcha_response.status(), Status::Ok);
        let captcha_body = captcha_response.into_json::<serde_json::Value>().await.unwrap();
        let captcha_id = captcha_body.get("captcha_id").unwrap().as_str().unwrap();
        let captcha_value = captcha_body.get("captcha_image").unwrap().as_str().unwrap();

        // 创建登录请求
        let login_data = serde_json::json!({
            "username": "admin",
            "password": "kenkone8282",
            "captchaId": captcha_id,
            "captcha": captcha_value
        });

        // 发送登录请求
        let response = client.post("/user/login")
            .header(ContentType::JSON)
            .body(login_data.to_string())
            .dispatch()
            .await;

        // 验证响应状态码
        assert_eq!(response.status(), Status::Ok);

        // 解析响应体（假设响应体是 JSON 并包含 token）
        let body = response.into_json::<serde_json::Value>().await.unwrap();
        assert!(body.get("token").is_some());
    }
}
