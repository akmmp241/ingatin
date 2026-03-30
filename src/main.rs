use crate::domain::reminder_job::ReminderJobStatus;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, SqlitePool};
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

mod config;
mod domain;
mod service;

const REMINDER_SYSTEM_PROMPT: &str = r#"
Kamu adalah 'Ingatin', asisten bot WhatsApp pribadi yang ramah, proaktif, dan ringkas. Tugasmu adalah membuat teks pengingat tugas berdasarkan data sistem.\r\n\r\nAturan:\r\n    1. Gunakan bahasa Indonesia yang santai tapi sopan (gunakan kata sapaan 'Halo' atau sebut nama jika ada).\r\n    2. Pesan harus singkat, padat, tidak lebih dari 3 kalimat, dan sangat mudah dibaca di layar HP.\r\n    3. Gunakan emoji yang relevan secukupnya untuk menyoroti urgensi.\r\n    4. Jangan menambahkan informasi fiktif di luar data yang diberikan.\r\n    5. Kirim respon dalam bentuk teks biasa untuk diparsing sebagi string.\r\n\r\nData Tugas:\r\n\r\n    - Judul Tugas: {title}\r\n    - Dateline: {deadline}\r\n    - Catatan Tambahan: {description} (abaikan jika kosong)\r\n    - Label: {label} (abaikan jika kosong)\r\n    - waktu ketika pengingat yang diminta saat ini: {remind_at}\r\n\r\nBuatkan pesan WhatsApp-nya dalam bentuk teks sekarang.
"#;

const EXTRACT_SYSTEM_PROMPT: &str = r#"
Kamu adalah AI pemroses bahasa alami untuk bot WhatsApp pengingat tugas pribadi.\r\n\r\nTujuanmu adalah menganalisis pesan pengguna, mengekstrak informasi tugas, dan menentukan kelengkapan data untuk disimpan ke dalam sistem.\r\n\r\nKonteks Waktu Saat Ini:\r\nWaktu Sistem: {current_time} (Gunakan ini sebagai titik acuan nol untuk menghitung hari dan jam).\r\n\r\nPesan Pengguna: '{user_message}'\r\n\r\nInformasi WAJIB yang dibutuhkan untuk sebuah tugas:\r\n1. Nama\/Deskripsi Tugas\r\n2. Waktu\/Tenggat Waktu (Deadline)\r\n3. Waktu kapan untuk diingatkan\r\n\r\nAnalisis pesan pengguna dan tentukan `action` berdasarkan aturan berikut:\r\n- \"SAVE\": Jika SEMUA informasi wajib sudah lengkap dan jelas.\r\n- \"ASK\": Jika ini adalah pesan terkait tugas, tetapi ada informasi wajib yang belum lengkap.\r\n- \"IRRELEVANT\": Jika pesan sama sekali tidak berhubungan dengan pembuatan atau manajemen tugas (misal: sapaan biasa, candaan, atau pertanyaan umum).\r\n\r\nBerikan respons HANYA dalam format JSON dengan struktur yang persis seperti ini:\r\n{\r\n\"action\": \"SAVE\" | \"ASK\" | \"IRRELEVANT\",\r\n\"extracted_data\": {\r\n\"title\": \"Nama atau deskripsi singkat tugas (String)\",\r\n\"description\": \"Detail tambahan jika ada, kembalikan null jika tidak ada (String|null)\",\r\n\"label\": \"Pengkategorian (contoh: Tugas, Event, dll), kembalikan null jika kurang bisa dikategorikan (String|null)\"\r\n\"deadline_at\": \"Waktu tenggat dalam format ISO 8601 YYYY-MM-DDTHH:MM:SSZ (String)\",\r\n\"reminders\": [\r\n\"Waktu pengingat 1 dalam format ISO 8601 YYYY-MM-DDTHH:MM:SS (String)\",\r\n\"Waktu pengingat 2 dalam format ISO 8601 YYYY-MM-DDTHH:MM:SS (String)\"\r\n]\r\n},\r\n\"reply_message\": \"string\"\r\n}\r\n\r\nAturan Ekstraksi:\r\n\r\n    1. Kembalikan HANYA format JSON mentah. Jangan ada teks pengantar, penutup, atau markdown code block (jangan gunakan ```json).\r\n\r\n    2. Jika pengguna meminta beberapa waktu pengingat (misal: 'ingetin H-1 dan 2 jam sebelumnya'), kalkulasi waktunya dan masukkan semuanya ke dalam array reminders.\r\n\r\n    3. Jika pengguna menyebutkan waktu tanpa AM\/PM, asumsikan waktu yang paling logis berdasarkan kebiasaan akademis (misal: 'jam 8' untuk tugas biasanya pagi, 'jam 8 malam' adalah 20:00).\r\n\r\nPanduan untuk `reply_message`:\r\n- Jika \"SAVE\": Berikan pesan konfirmasi singkat bahwa tugas akan disimpan. Bila dalam pesan ini ada waktu ubah jadi WIB (+07:00).\r\n- Jika \"ASK\": Tanyakan secara natural spesifik informasi apa yang masih kurang (misal: \"Tugasnya mau diingatkan jam berapa?\").\r\n- Jika \"IRRELEVANT\": Balas dengan sopan sesuai konteks pesan pengguna, atau ingatkan bahwa kamu adalah bot pengingat tugas.
"#;

#[derive(Clone)]
struct AppState {
    pub db: Arc<SqlitePool>,
    pub client: reqwest::Client,
    pub api_key: String,
    pub model_type: String,
    pub waha_api_url: String,
    pub waha_api_key: String,
    pub waha_session: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env");
    let tick_interval = std::env::var("TICK_INTERVAL")
        .unwrap_or_else(|_| "60".to_string())
        .parse::<i8>()
        .expect("TICK_INTERVAL must be a number");
    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY is not set in .env");
    let model_type =
        std::env::var("GEMINI_MODEL_TYPE").expect("GEMINI_MODEL_TYPE is not set in .env");
    let waha_api_url = std::env::var("WAHA_API_URL").expect("WAHA_API_URL must be set");
    let waha_api_key = std::env::var("WAHA_API_KEY").expect("WAHA_API_KEY must be set");
    let waha_session = std::env::var("WAHA_SESSION").expect("WAHA_SESSION must be set");

    let db = config::get_db_pool(db_url).await?;

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(50)
        .timeout(Duration::from_secs(15))
        .build()?;

    let app = Arc::new(AppState {
        db: Arc::new(db.clone()),
        client: client.clone(),
        api_key,
        model_type,
        waha_api_url,
        waha_api_key,
        waha_session,
    });

    tokio::join!(
        start_webserver(app.clone()),
        start_scheduler(app, tick_interval as u64)
    );

    Ok(())
}

async fn start_scheduler(config: Arc<AppState>, interval: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval));

    tracing::info!("start scheduler");

    loop {
        interval.tick().await;

        let schedules = get_pending_schedules(&config.db).await.unwrap_or_default();

        tracing::info!(schedules = ?schedules);

        for schedule in schedules {
            let target = &schedule.target.clone();
            let job_id = &schedule.job_id.clone();

            let message = generate_reminder_msg(config.clone(), schedule).await;

            // gagal membuat reminder message
            if let Err(e) = message {
                tracing::error!("{}", e);
                continue;
            }

            let msg = message.unwrap();

            match send_waha_message(config.clone(), &msg, target).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("{}", e);
                    continue;
                }
            }

            set_reminder_to_sent(&config.db, job_id).await.unwrap();
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, FromRow)]
struct TaskWithReminder {
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
async fn get_pending_schedules(pool: &SqlitePool) -> Result<Vec<TaskWithReminder>, sqlx::Error> {
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
                   r.status as "status: domain::reminder_job::ReminderJobStatus"
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

async fn generate_reminder_msg(
    config: Arc<AppState>,
    task: TaskWithReminder,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        config.model_type, config.api_key
    );

    let system_instruction = REMINDER_SYSTEM_PROMPT;
    let prompt = serde_json::to_string(&task).map_err(|e| {
        let err_msg = e.to_string();
        tracing::error!(error = %err_msg, "failed to serialize task");
        e
    })?;

    let payload = json!({
        "systemInstruction": {
            "parts": [{"text": system_instruction}]
        },
        "contents": [{
            "role": "user",
            "parts": [{"text": prompt}]
        }],
        "generationConfig": {
            "responseMimeType": "text/plain",
            "thinkingConfig": {
                "thinkingLevel": "MINIMAL",
            },
            "mediaResolution": "MEDIA_RESOLUTION_LOW",
        },
        "safetySettings": [
            {
                "category": "HARM_CATEGORY_HARASSMENT",
                "threshold": "BLOCK_LOW_AND_ABOVE"
            },
            {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "threshold": "BLOCK_LOW_AND_ABOVE"
            },
            {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "threshold": "BLOCK_LOW_AND_ABOVE"
            },
            {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "threshold": "BLOCK_LOW_AND_ABOVE"
            },
        ],
    });

    let response = config.client.post(&url).json(&payload).send().await?;

    if !response.status().is_success() {
        let status = response.status().to_string();
        let detail = response.text().await?;

        tracing::error!(
            status = %status,
            detail = %detail,
            "failed to request gemini api"
        );
        return Err(Box::from("failed to request gemini api"));
    }

    let response_body: Value = response.json().await?;

    if let Some(text) = response_body["candidates"][0]["content"]["parts"][0]["text"].as_str() {
        Ok(text.to_string())
    } else {
        tracing::warn!("invalid response payload");
        Err(Box::from("invalid response payload"))
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct SendTextReq {
    #[serde(rename = "chatId")]
    chat_id: String,
    text: String,
    session: String,
}

async fn send_waha_message(
    config: Arc<AppState>,
    text: &str,
    target: &str,
) -> Result<(), Box<dyn Error>> {
    let url = format!("{}/api/sendText", config.waha_api_url);

    let payload = SendTextReq {
        chat_id: format!("{}@c.us", sanitize_target(target)),
        text: text.to_string(),
        session: config.waha_session.clone(),
    };

    let fresh_client = reqwest::Client::new();

    let resp = fresh_client
        .post(&url)
        .header("X-Api-Key", config.waha_api_key.as_str())
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

async fn set_reminder_to_sent(
    config: &SqlitePool,
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query!("UPDATE reminder_jobs SET status = 'SENT' WHERE id = ?", id)
        .execute(config)
        .await
        .map_err(|e: sqlx::Error| {
            let err_msg = e.to_string();
            tracing::error!(error = err_msg, "error from database");
            e
        })?;

    Ok(())
}

async fn start_webserver(app: Arc<AppState>) {
    let app = Router::new()
        .route("/webhook/waha", post(waha_handler))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("Failed to bind");

    tracing::info!("server start in :3000");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct WahaMsgPayload {
    pub event: String,
    pub from_me: bool,
    pub message: String,
    pub sender: String,
}

impl Into<WahaMsgPayload> for Value {
    fn into(self) -> WahaMsgPayload {
        let event = self["event"].as_str().unwrap_or("").to_string();
        let from_me = self["payload"]["fromMe"]
            .as_str()
            .unwrap_or("")
            .parse::<bool>()
            .unwrap_or(false);
        let message = self["payload"]["body"].as_str().unwrap_or("").to_string();
        let sender = self["payload"]["_data"]["Info"]["SenderAlt"]
            .as_str()
            .unwrap_or("")
            // ambil nomornya saja
            .trim_end_matches("@s.whatsapp.net")
            .to_string();

        WahaMsgPayload {
            event,
            from_me,
            message,
            sender,
        }
    }
}

async fn waha_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    tracing::info!("received waha payload");

    let req: WahaMsgPayload = payload.into();
    let sender = &req.sender.clone();

    let llm_extracted = match parse_msg(state.clone(), req).await {
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
        let _ = insert_task_and_reminder(&state.db, extracted_data, sender)
            .await
            .unwrap();
    }

    match send_waha_message(state.clone(), &llm_extracted.reply_message, sender).await {
        Ok(_) => {
            tracing::info!("successfully sent waha message");
        }
        Err(e) => {
            tracing::error!("{}", e);
        }
    }

    (StatusCode::OK, "success".to_string()).into_response()
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct LlmExtracted {
    pub action: LlmAction,
    pub extracted_data: Option<ExtractedTask>,
    pub reply_message: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "UPPERCASE")]
enum LlmAction {
    Save,
    Ask,
    Irrelevant,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct ExtractedTask {
    pub title: String,
    pub description: Option<String>,
    pub label: Option<String>,
    pub deadline_at: String,
    pub reminders: Vec<String>,
}

/// minta llm untuk memproses pesan dari user dan mengubah ke structured json
async fn parse_msg(
    config: Arc<AppState>,
    req: WahaMsgPayload,
) -> Result<LlmExtracted, Box<dyn std::error::Error>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        config.model_type, config.api_key
    );

    let current_time = Utc::now();
    let message = req.message;
    let user_prompt = json!(
        {"current_time": current_time, "user_message": message}
    );

    let system_instruction = EXTRACT_SYSTEM_PROMPT;

    let prompt = serde_json::to_string(&user_prompt).map_err(|e| {
        let err_msg = e.to_string();
        tracing::error!(error = %err_msg, "failed to serialize user message");
        e
    })?;

    let payload = json!({
        "systemInstruction": {
            "parts": [{"text": system_instruction}]
        },
        "contents": [{
            "role": "user",
            "parts": [{"text": &prompt}]
        }],
        "generationConfig": {
            "responseMimeType": "text/plain",
            "thinkingConfig": {
                "thinkingLevel": "HIGH",
            },
            "mediaResolution": "MEDIA_RESOLUTION_LOW",
        },
        "safetySettings": [
            {
                "category": "HARM_CATEGORY_HARASSMENT",
                "threshold": "BLOCK_LOW_AND_ABOVE"
            },
            {
                "category": "HARM_CATEGORY_HATE_SPEECH",
                "threshold": "BLOCK_LOW_AND_ABOVE"
            },
            {
                "category": "HARM_CATEGORY_SEXUALLY_EXPLICIT",
                "threshold": "BLOCK_LOW_AND_ABOVE"
            },
            {
                "category": "HARM_CATEGORY_DANGEROUS_CONTENT",
                "threshold": "BLOCK_LOW_AND_ABOVE"
            },
        ],
    });

    let response = config.client.post(&url).json(&payload).send().await?;

    if !response.status().is_success() {
        let status = response.status().to_string();
        let detail = response.text().await?;

        tracing::error!(
            status = %status,
            detail = %detail,
            "failed to request gemini api"
        );
        return Err(Box::from("failed to request gemini api"));
    }

    let response_body: Value = response.json().await?;

    if let Some(text) = response_body["candidates"][0]["content"]["parts"][0]["text"].as_str() {
        let llm_extracted = serde_json::from_str::<LlmExtracted>(text)?;

        Ok(llm_extracted)
    } else {
        tracing::warn!("invalid response payload");
        Err(Box::from("invalid response payload"))
    }
}

async fn insert_task_and_reminder(
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
