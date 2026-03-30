use crate::domain::reminder_job::ReminderJobStatus;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

#[derive(Deserialize, Serialize, Clone, Debug, FromRow)]
pub struct TaskWithReminder {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub label: Option<String>,
    pub target: String,
    pub dateline: DateTime<Utc>,
    pub job_id: String,
    pub task_id: String,
    pub remind_at: DateTime<Utc>,
    pub status: ReminderJobStatus,
}

/// ambil pending jobs
pub async fn get_pending_schedules(
    pool: &SqlitePool,
) -> Result<Vec<TaskWithReminder>, sqlx::Error> {
    let now = Utc::now();

    let schedules = sqlx::query_as!(
        TaskWithReminder,
        r#"
            select t.id,
                   t.title,
                   t.description,
                   t.label,
                   t.target,
                   t.dateline as "dateline: chrono::DateTime<chrono::Utc>",
                   r.id as job_id,
                   r.task_id,
                   r.remind_at as "remind_at: chrono::DateTime<chrono::Utc>",
                   r.status as "status: crate::domain::reminder_job::ReminderJobStatus"
            from tasks t
                     inner join reminder_jobs r on t.id = r.task_id
            where r.remind_at <= ?
              and r.status = 'PENDING';
            "#,
        now
    )
    .fetch_all(pool)
    .await?;

    Ok(schedules)
}

pub async fn set_reminder_to_sent(
    pool: &SqlitePool,
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query!("UPDATE reminder_jobs SET status = 'SENT' WHERE id = ?", id)
        .execute(pool)
        .await
        .map_err(|e: sqlx::Error| {
            let err_msg = e.to_string();
            tracing::error!(error = err_msg, "error from database");
            e
        })?;

    Ok(())
}
