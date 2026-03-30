use crate::client::{gemini, waha};
use crate::config::AppState;
use crate::repository::reminder_repository;
use std::sync::Arc;
use std::time::Duration;

pub async fn start(state: Arc<AppState>, interval: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval));

    tracing::info!("start scheduler");

    loop {
        interval.tick().await;

        let schedules = reminder_repository::get_pending_schedules(&state.db)
            .await
            .unwrap_or_default();

        tracing::info!(schedules = ?schedules);

        for schedule in schedules {
            let target = &schedule.target.clone();
            let job_id = &schedule.job_id.clone();

            let message = gemini::generate_reminder_msg(state.clone(), schedule).await;

            // gagal membuat reminder message
            if let Err(e) = message {
                tracing::error!("{}", e);
                continue;
            }

            let msg = message.unwrap();

            match waha::send_message(state.clone(), &msg, target).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("{}", e);
                    continue;
                }
            }

            reminder_repository::set_reminder_to_sent(&state.db, job_id)
                .await
                .unwrap();
        }
    }
}
