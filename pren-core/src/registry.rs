//! # Prompt Registry
//!
//! This module defines the core traits and structures for prompt storage and management.
//!
//! The main components are:
//! - [`PromptStorage`] trait - Defines the interface for storing and retrieving prompts
//! - [`PromptFile`] struct - Represents the serialized format of prompts on disk

use crate::prompt::Prompt;
use serde::{Deserialize, Serialize};

/// Represents a prompt stored in a TOML file.
#[derive(Debug, Deserialize, Serialize)]
pub struct PromptFile {
    /// The name of the prompt.
    pub name: String,
    /// Tags associated with the prompt.
    pub tags: Vec<String>,
    /// The type of prompt ("simple" or "template").
    pub prompt_type: String,
    /// The content of the prompt.
    pub content: String,
}

/// A trait for storing and retrieving prompts.
///
/// This trait defines the interface for prompt storage implementations.
/// Implementors can store prompts in various backends such as files, databases, etc.
pub trait PromptStorage {
    /// The error type for storage operations.
    type Error: std::error::Error + Send + Sync;

    /// Saves a prompt to the storage.
    fn save_prompt(&self, prompt: &Prompt) -> Result<(), Self::Error>;
    
    /// Retrieves a prompt by name.
    fn get_prompt(&self, name: &str) -> Result<Option<Prompt>, Self::Error>;
    
    /// Retrieves all prompts.
    fn get_prompts(&self) -> Result<Vec<Prompt>, Self::Error>;
    
    /// Deletes a prompt by name.
    fn delete_prompt(&self, name: &str) -> Result<(), Self::Error>;
    
    /// Retrieves prompts that have any of the specified tags.
    fn get_prompts_by_tag(&self, tags: &[String]) -> Result<Vec<Prompt>, Self::Error>;
}
