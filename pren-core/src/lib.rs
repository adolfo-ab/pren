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
//! - [`registry`] - Prompt storage traits and file format definitions
//!
//! # Examples
//!
//! ```rust
//! use pren_core::prompt::Prompt;
//! use pren_core::file_storage::FileStorage;
//! use std::path::PathBuf;
//!
//! // Create a prompt
//! let prompt = Prompt::new_simple(
//!     "greeting".to_string(),
//!     "Hello, world!".to_string(),
//!     vec!["example".to_string()]
//! );
//!
//! // Save it to file storage
//! let storage = FileStorage {
//!     base_path: PathBuf::from("./prompts"),
//! };
//! storage.save_prompt(&prompt).unwrap();
//! ```

mod file_storage;
mod parser;
pub mod prompt;
pub mod registry;
