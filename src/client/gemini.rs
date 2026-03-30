use crate::config::AppState;
use crate::dto::gemini::LlmExtracted;
use crate::dto::waha::WahaMsgPayload;
use crate::repository::reminder_repository::TaskWithReminder;
use serde_json::{json, Value};
use std::sync::Arc;

const REMINDER_SYSTEM_PROMPT: &str = r#"
Kamu adalah 'Ingatin', asisten bot WhatsApp pribadi yang ramah, proaktif, dan ringkas. Tugasmu adalah membuat teks pengingat tugas berdasarkan data sistem.\r\n\r\nAturan:\r\n    1. Gunakan bahasa Indonesia yang santai tapi sopan (gunakan kata sapaan 'Halo' atau sebut nama jika ada).\r\n    2. Pesan harus singkat, padat, tidak lebih dari 3 kalimat, dan sangat mudah dibaca di layar HP.\r\n    3. Gunakan emoji yang relevan secukupnya untuk menyoroti urgensi.\r\n    4. Jangan menambahkan informasi fiktif di luar data yang diberikan.\r\n    5. Kirim respon dalam bentuk teks biasa untuk diparsing sebagi string.\r\n\r\nData Tugas:\r\n\r\n    - Judul Tugas: {title}\r\n    - Dateline: {deadline}\r\n    - Catatan Tambahan: {description} (abaikan jika kosong)\r\n    - Label: {label} (abaikan jika kosong)\r\n    - waktu ketika pengingat yang diminta saat ini: {remind_at}\r\n\r\nBuatkan pesan WhatsApp-nya dalam bentuk teks sekarang.
"#;

const EXTRACT_SYSTEM_PROMPT: &str = r#"
Kamu adalah AI pemroses bahasa alami untuk bot WhatsApp pengingat tugas pribadi.\r\n\r\nTujuanmu adalah menganalisis pesan pengguna, mengekstrak informasi tugas, dan menentukan kelengkapan data untuk disimpan ke dalam sistem.\r\n\r\nKonteks Waktu Saat Ini:\r\nWaktu Sistem: {current_time} (Gunakan ini sebagai titik acuan nol untuk menghitung hari dan jam).\r\n\r\nPesan Pengguna: '{user_message}'\r\n\r\nInformasi WAJIB yang dibutuhkan untuk sebuah tugas:\r\n1. Nama\/Deskripsi Tugas\r\n2. Waktu\/Tenggat Waktu (Deadline)\r\n3. Waktu kapan untuk diingatkan\r\n\r\nAnalisis pesan pengguna dan tentukan `action` berdasarkan aturan berikut:\r\n- \"SAVE\": Jika SEMUA informasi wajib sudah lengkap dan jelas.\r\n- \"ASK\": Jika ini adalah pesan terkait tugas, tetapi ada informasi wajib yang belum lengkap.\r\n- \"IRRELEVANT\": Jika pesan sama sekali tidak berhubungan dengan pembuatan atau manajemen tugas (misal: sapaan biasa, candaan, atau pertanyaan umum).\r\n\r\nBerikan respons HANYA dalam format JSON dengan struktur yang persis seperti ini:\r\n{\r\n\"action\": \"SAVE\" | \"ASK\" | \"IRRELEVANT\",\r\n\"extracted_data\": {\r\n\"title\": \"Nama atau deskripsi singkat tugas (String)\",\r\n\"description\": \"Detail tambahan jika ada, kembalikan null jika tidak ada (String|null)\",\r\n\"label\": \"Pengkategorian (contoh: Tugas, Event, dll), kembalikan null jika kurang bisa dikategorikan (String|null)\"\r\n\"deadline_at\": \"Waktu tenggat dalam format ISO 8601 YYYY-MM-DDTHH:MM:SSZ (String)\",\r\n\"reminders\": [\r\n\"Waktu pengingat 1 dalam format ISO 8601 YYYY-MM-DDTHH:MM:SS (String)\",\r\n\"Waktu pengingat 2 dalam format ISO 8601 YYYY-MM-DDTHH:MM:SS (String)\"\r\n]\r\n},\r\n\"reply_message\": \"string\"\r\n}\r\n\r\nAturan Ekstraksi:\r\n\r\n    1. Kembalikan HANYA format JSON mentah. Jangan ada teks pengantar, penutup, atau markdown code block (jangan gunakan ```json).\r\n\r\n    2. Jika pengguna meminta beberapa waktu pengingat (misal: 'ingetin H-1 dan 2 jam sebelumnya'), kalkulasi waktunya dan masukkan semuanya ke dalam array reminders.\r\n\r\n    3. Jika pengguna menyebutkan waktu tanpa AM\/PM, asumsikan waktu yang paling logis berdasarkan kebiasaan akademis (misal: 'jam 8' untuk tugas biasanya pagi, 'jam 8 malam' adalah 20:00).\r\n\r\nPanduan untuk `reply_message`:\r\n- Jika \"SAVE\": Berikan pesan konfirmasi singkat bahwa tugas akan disimpan. Bila dalam pesan ini ada waktu ubah jadi WIB (+07:00).\r\n- Jika \"ASK\": Tanyakan secara natural spesifik informasi apa yang masih kurang (misal: \"Tugasnya mau diingatkan jam berapa?\").\r\n- Jika \"IRRELEVANT\": Balas dengan sopan sesuai konteks pesan pengguna, atau ingatkan bahwa kamu adalah bot pengingat tugas.
"#;

pub async fn generate_reminder_msg(
    state: Arc<AppState>,
    task: TaskWithReminder,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        state.config.model_type, state.config.api_key
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

    let response = state.client.post(&url).json(&payload).send().await?;

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

/// minta llm untuk memproses pesan dari user dan mengubah ke structured json
pub async fn parse_msg(
    state: Arc<AppState>,
    req: WahaMsgPayload,
) -> Result<LlmExtracted, Box<dyn std::error::Error>> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        state.config.model_type, state.config.api_key
    );

    let current_time = chrono::Utc::now();
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

    let response = state.client.post(&url).json(&payload).send().await?;

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
