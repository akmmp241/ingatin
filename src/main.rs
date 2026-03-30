use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

mod client;
mod config;
mod domain;
mod dto;
mod handler;
mod repository;
mod scheduler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env");
    let tick_interval = std::env::var("TICK_INTERVAL")
        .unwrap_or_else(|_| "60".to_string())
        .parse::<i8>()
        .expect("TICK_INTERVAL must be a number");

    let app_config = config::AppConfig::from_env();
    let db = config::get_db_pool(db_url).await?;

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(50)
        .timeout(Duration::from_secs(15))
        .build()?;

    let state = Arc::new(config::AppState {
        db: Arc::new(db),
        client,
        config: app_config,
    });

    tokio::join!(
        start_webserver(state.clone()),
        scheduler::runner::start(state, tick_interval as u64)
    );

    Ok(())
}

async fn start_webserver(state: Arc<config::AppState>) {
    let app = handler::webhook::routes(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind");

    tracing::info!("server start in :3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
