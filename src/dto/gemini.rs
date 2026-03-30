use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct LlmExtracted {
    pub action: LlmAction,
    pub extracted_data: Option<ExtractedTask>,
    pub reply_message: String,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq, Debug)]
#[serde(rename_all = "UPPERCASE")]
pub enum LlmAction {
    Save,
    Ask,
    Irrelevant,
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct ExtractedTask {
    pub title: String,
    pub description: Option<String>,
    pub label: Option<String>,
    pub deadline_at: String,
    pub reminders: Vec<String>,
}
