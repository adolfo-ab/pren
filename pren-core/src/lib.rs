//! # pren Core
//!
//! This crate provides the core functionality for the PREN (Prompt Engine) system.
//!
//! pren is a prompt management system that allows you to organize, store, and use prompts
//! with template capabilities for dynamic content generation.
//!
//! # Modules
//!
//! - [`file_storage`] - File-based storage implementation for prompts
//! - [`parser`] - Template parsing functionality
//! - [`prompt`] - Core prompt data structures and functionality
//! - [`storage`] - Prompt storage traits and file format definitions
//!
//! # Examples
//!
//! ```rust
//! use pren_core::prompt::{Prompt, PromptMetadata};
//! use pren_core::file_storage::FileStorage;
//! use pren_core::storage::PromptStorage;
//! use std::path::PathBuf;
//! use tempfile::TempDir;
//!
//! // Create a temporary directory for our tests
//! let temp_dir = TempDir::new().unwrap();
//!
//! // Create a prompt
//! let metadata = PromptMetadata::new("greeting".to_string(), None, vec!["example".to_string()]);
//! let prompt = Prompt::new(metadata, "Hello, world!".to_string());
//!
//! // Save it to file storage
//! let storage = FileStorage {
//!     base_path: temp_dir.path().to_path_buf(),
//! };
//! storage.save_prompt(&prompt).expect("Failed to save prompt");
//! ```

pub mod file_storage;
pub mod llm;
pub mod parser;
pub mod prompt;
pub mod storage;
