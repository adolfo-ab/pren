use std::error::Error;
use serde::{Deserialize, Serialize};
use crate::file_storage::FileStorageError;
use crate::prompt::Prompt;

#[derive(Debug, Deserialize, Serialize)]
pub struct PromptFile {
    pub name: String,
    pub tags: Vec<String>,
    #[serde(rename = "type")]
    pub prompt_type: String,
    pub content: String,
}

pub trait PromptStorage {
    fn save_prompt(&self, prompt: &Prompt) -> Result<(), FileStorageError>;
    fn load_prompt(&self, name: &str) -> Result<Option<Prompt>, Box<dyn Error>>;
    fn list_prompts(&self) -> Result<Vec<String>, Box<dyn Error>>;
    fn delete_prompt(&self, name: &str) -> Result<(), Box<dyn Error>>;
    fn search_prompts_by_tags(&self, tags: &[String]) -> Result<Vec<Prompt>, Box<dyn Error>>;
}