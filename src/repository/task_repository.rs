use crate::dto::gemini::ExtractedTask;
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn insert_task_and_reminder(
    pool: &SqlitePool,
    data: ExtractedTask,
    target: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let id = Uuid::new_v4().to_string();

    let _res_task = sqlx::query!(
        r#"
        INSERT INTO tasks (id, title, description, label, target, dateline)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
        id,
        data.title,
        data.description,
        data.label,
        target,
        data.deadline_at
    )
    .execute(pool)
    .await?;

    for reminder in data.reminders {
        let reminder_id = Uuid::new_v4().to_string();

        sqlx::query!(
            r#"
            INSERT INTO reminder_jobs (id, task_id, remind_at)
            VALUES (?, ?, ?)
            "#,
            reminder_id,
            id,
            reminder
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}
