use crate::client::{gemini, waha};
use crate::config::AppState;
use crate::dto::gemini::LlmAction;
use crate::dto::waha::WahaMsgPayload;
use crate::repository::task_repository;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;
use std::sync::Arc;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/webhook/waha", post(waha_handler))
        .with_state(state)
}

async fn waha_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    tracing::info!("received waha payload");

    let req: WahaMsgPayload = payload.into();
    let sender = &req.sender.clone();

    let llm_extracted = match gemini::parse_msg(state.clone(), req).await {
        Ok(extracted) => extracted,
        Err(e) => {
            tracing::error!("error while parse msg: {}", e.to_string());
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
        }
    };

    // pastikan llm tidak merespon selain 'SAVE' dan extracted data null
    // baru insert
    if let Some(extracted_data) = llm_extracted.extracted_data
        && llm_extracted.action == LlmAction::Save
    {
        let _ = task_repository::insert_task_and_reminder(&state.db, extracted_data, sender)
            .await
            .unwrap();
    }

    match waha::send_message(state.clone(), &llm_extracted.reply_message, sender).await {
        Ok(_) => {
            tracing::info!("successfully sent waha message");
        }
        Err(e) => {
            tracing::error!("{}", e);
        }
    }

    (StatusCode::OK, "success".to_string()).into_response()
}
