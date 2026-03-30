use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Error, SqlitePool};

pub async fn get_db_pool(url: String) -> Result<SqlitePool, Error> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
}