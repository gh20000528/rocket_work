use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use sqlx::Pool;
use sqlx::Error;
use sqlx::postgres::Postgres;

pub async fn init_db() -> PgPool {
    dotenv::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool")
}


pub async fn test_db_connection(pool: &Pool<Postgres>) -> Result<(), Error> {
    // 嘗試從數據庫獲取當前時間來測試連接
    sqlx::query!("SELECT NOW()").fetch_one(pool).await.map(|_| ())
}
