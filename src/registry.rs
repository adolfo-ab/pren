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
    fn get_prompt(&self, name: &str) -> Result<Option<Prompt>, FileStorageError>;
    fn get_prompts(&self) -> Result<Vec<Prompt>, FileStorageError>;
    fn delete_prompt(&self, name: &str) -> Result<(), FileStorageError>;
    fn get_prompts_by_tag(&self, tags: &[String]) -> Result<Vec<Prompt>, FileStorageError>;
}