use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<JsonSchema>,
}

#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct JsonSchema {
    pub name: String,
    pub schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
