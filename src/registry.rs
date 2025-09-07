use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PromptFile {
    pub name: String,
    pub tags: Vec<String>,
    #[serde(rename = "type")]
    pub prompt_type: String,
    pub content: String,
}