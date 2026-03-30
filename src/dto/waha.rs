use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct WahaMsgPayload {
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

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct SendTextReq {
    #[serde(rename = "chatId")]
    pub chat_id: String,
    pub text: String,
    pub session: String,
}
