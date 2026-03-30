use crate::config::AppState;
use crate::dto::waha::SendTextReq;
use reqwest::header;
use std::error::Error;
use std::sync::Arc;

pub async fn send_message(
    state: Arc<AppState>,
    text: &str,
    target: &str,
) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/api/sendText", state.config.waha_api_url);

    let payload = SendTextReq {
        chat_id: format!("{}@c.us", sanitize_target(target)),
        text: text.to_string(),
        session: state.config.waha_session.clone(),
    };

    let resp = state
        .client
        .post(&url)
        .header("X-Api-Key", state.config.waha_api_key.as_str())
        .header(header::ACCEPT, "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            let err_msg = e.to_string();
            tracing::error!(error = %err_msg, "failed to call waha api");

            if let Some(source) = e.source() {
                println!("source: {:?}", source);
            }

            e
        })?;

    if !resp.status().is_success() {
        let status = resp.status().to_string();
        let detail = resp.text().await?;

        tracing::error!(
            status = %status,
            detail = %detail,
            "error while sending waha message"
        );

        return Err(Box::from("failed to send waha message"));
    }

    Ok(())
}

/// menghapus + pada nomor hp target
///
/// contoh '+6285159958218' jadi '6285159958218'
fn sanitize_target(target: &str) -> &str {
    target.trim_start_matches("+")
}
