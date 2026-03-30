use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Error, SqlitePool};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub model_type: String,
    pub waha_api_url: String,
    pub waha_api_key: String,
    pub waha_session: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY is not set in .env"),
            model_type: std::env::var("GEMINI_MODEL_TYPE")
                .expect("GEMINI_MODEL_TYPE is not set in .env"),
            waha_api_url: std::env::var("WAHA_API_URL").expect("WAHA_API_URL must be set"),
            waha_api_key: std::env::var("WAHA_API_KEY").expect("WAHA_API_KEY must be set"),
            waha_session: std::env::var("WAHA_SESSION").expect("WAHA_SESSION must be set"),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub client: reqwest::Client,
    pub config: AppConfig,
}

pub async fn get_db_pool(url: String) -> Result<SqlitePool, Error> {
    SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
}