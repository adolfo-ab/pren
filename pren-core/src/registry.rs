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
    type Error: std::error::Error + Send + Sync;

    fn save_prompt(&self, prompt: &Prompt) -> Result<(), Self::Error>;
    fn get_prompt(&self, name: &str) -> Result<Option<Prompt>, Self::Error>;
    fn get_prompts(&self) -> Result<Vec<Prompt>, Self::Error>;
    fn delete_prompt(&self, name: &str) -> Result<(), Self::Error>;
    fn get_prompts_by_tag(&self, tags: &[String]) -> Result<Vec<Prompt>, Self::Error>;
}