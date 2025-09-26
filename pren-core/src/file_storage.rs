//! # File Storage
//!
//! This module provides functionality for storing and retrieving prompts from the local filesystem.
//! Prompts are stored as individual markdown files with YAML frontmatter in a specified directory.
//!
//! The main component of this module is the [`FileStorage`] struct, which implements the
//! [`PromptStorage`] trait to provide persistent storage capabilities for prompts.
//!
//! # Examples
//!
//! ```rust
//! use pren_core::file_storage::FileStorage;
//! use pren_core::prompt::{Prompt, PromptMetadata};
//! use pren_core::storage::PromptStorage;
//! use std::path::PathBuf;
//! use tempfile::TempDir;
//!
//! // Create a temporary directory for our tests
//! let temp_dir = TempDir::new().unwrap();
//!
//! // Create a new file storage instance
//! let storage = FileStorage {
//!     base_path: temp_dir.path().to_path_buf(),
//! };
//!
//! // Create a simple prompt
//! let metadata = PromptMetadata::new("greeting".to_string(), None, vec!["example".to_string()]);
//! let prompt = Prompt::new(metadata, "Hello, world!".to_string());
//!
//! // Save the prompt to disk
//! storage.save_prompt(&prompt).expect("Failed to save prompt");
//! ```

use crate::prompt::{ParseTemplateError, Prompt, PromptMetadata};
#[cfg(test)]
use crate::prompt::PromptTemplate;
use crate::storage::PromptStorage;
use std::error::Error;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::{fmt, fs, io};
use walkdir::WalkDir;

#[derive(Debug)]
pub enum FileStorageError {
    IoError(io::Error),
    SerializationError(serde_frontmatter::SerdeFMError),
    InvalidBasePath(String),
    PromptNotFound(String),
    InvalidPromptType(String),
    ParseTemplateError(ParseTemplateError),
}

impl fmt::Display for FileStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileStorageError::IoError(err) => write!(f, "IO error: {}", err),
            FileStorageError::SerializationError(err) => {
                write!(f, "Serialization error: {:?}", err)
            },
            FileStorageError::InvalidBasePath(path) => write!(f, "Invalid base path: {}", path),
            FileStorageError::PromptNotFound(path) => write!(f, "Prompt not found: {}", path),
            FileStorageError::InvalidPromptType(prompt_type) => write!(
                f,
                "Invalid prompt type, must be 'simple' or 'template': {}",
                prompt_type
            ),
            FileStorageError::ParseTemplateError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for FileStorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FileStorageError::IoError(err) => Some(err),
            FileStorageError::ParseTemplateError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for FileStorageError {
    fn from(err: io::Error) -> Self {
        FileStorageError::IoError(err)
    }
}

impl From<serde_frontmatter::SerdeFMError> for FileStorageError {
    fn from(err: serde_frontmatter::SerdeFMError) -> Self {
        FileStorageError::SerializationError(err)
    }
}

impl From<ParseTemplateError> for FileStorageError {
    fn from(err: ParseTemplateError) -> Self {
        FileStorageError::ParseTemplateError(err)
    }
}

/// A local file storage for Prompts.
///
/// Saves prompts as markdown files with YAML frontmatter in the specified directory.
pub struct FileStorage {
    /// The base directory where prompt files are stored.
    pub base_path: PathBuf,
}

impl PromptStorage for FileStorage {
    type Error = FileStorageError;

    /// Saves a prompt in the local file system.
    ///
    /// This function tries to save a prompt in a markdown file with YAML frontmatter.
    /// If `base_path` doesn't exist, it is created first.
    /// If the file already exists, it is overwritten.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to be saved.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the prompt is saved correctly.
    /// * `FileStorageError::InvalidBasePath` - If prompt cannot be saved because `base_path` is not a directory.
    fn save_prompt(&self, prompt: &Prompt) -> Result<(), FileStorageError> {
        self.ensure_base_directory_exists()?;

        let file_path = self.base_path.join(format!("{}.md", prompt.metadata.name));

        match serde_frontmatter::serialize(&prompt.metadata, prompt.content.as_str()) {
            Ok(serialized_data) => {
                fs::write(file_path, serialized_data)?;
                Ok(())
            }
            Err(e) => Err(FileStorageError::SerializationError(e))
        }
    }

    /// Gets a prompt given its name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the prompt to be retrieved.
    ///
    /// # Returns
    ///
    /// * `Ok(Prompt)` - If the prompt is found.
    /// * `FileStorageError` - If there was an error reading or parsing the prompt, or if the prompt doesn't exist.
    fn get_prompt(&self, name: &str) -> Result<Prompt, FileStorageError> {
        let file_path = self.base_path.join(format!("{}.md", name));
        if !file_path.exists() {
            return Err(FileStorageError::PromptNotFound(
                file_path.display().to_string(),
            ));
        }

        let content = fs::read_to_string(file_path)?;
        let (metadata, raw_content): (PromptMetadata, String)  = serde_frontmatter::deserialize(content.as_str())?;
        let content = raw_content.trim_start().to_string();

        Ok(Prompt::new(metadata, content))
    }

    /// Gets all prompts stored in the base directory.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Prompt>)` - A vector containing all prompts found in the storage.
    /// * `FileStorageError` - If there was an error reading or parsing any prompt.
    fn get_prompts(&self) -> Result<Vec<Prompt>, FileStorageError> {
        let mut prompts = Vec::new();

        // Walk through the base directory
        for entry in self.get_md_files()? {
            let file_path = entry.path();

            // Read and parse the file
            let content = fs::read_to_string(file_path)?;
            let (metadata, raw_content): (PromptMetadata, String)  = serde_frontmatter::deserialize(content.as_str())?;
            let content = raw_content.trim_start().to_string();

            prompts.push(Prompt::new(metadata, content));
        }

        Ok(prompts)
    }

    /// Gets all prompts that have any of the specified tags.
    ///
    /// # Arguments
    ///
    /// * `tags` - A slice of tag names to search for.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Prompt>)` - A vector containing all prompts that match any of the tags.
    /// * `FileStorageError` - If there was an error reading or parsing any prompt.
    fn get_prompts_by_tag(&self, tags: &[String]) -> Result<Vec<Prompt>, FileStorageError> {
        let mut prompts = Vec::new();

        // Walk through the base directory
        for entry in self.get_md_files()? {
            let file_path = entry.path();

            // Read and parse the file
            let content = fs::read_to_string(file_path)?;
            let (metadata, raw_content): (PromptMetadata, String)  = serde_frontmatter::deserialize(content.as_str())?;
            let content = raw_content.trim_start().to_string();

            let prompt = Prompt::new(metadata, content);

            // Check if any of the prompt's tags match any of the requested tags
            if prompt
                .metadata.tags
                .iter()
                .any(|prompt_tag| tags.contains(prompt_tag))
            {
                prompts.push(prompt);
            }
        }

        Ok(prompts)
    }

    /// Deletes a prompt given its name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the prompt to be deleted.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the prompt was successfully deleted or didn't exist.
    /// * `FileStorageError` - If there was an error deleting the file or the file didn't exist.
    fn delete_prompt(&self, name: &str) -> Result<(), FileStorageError> {
        let file_path = self.base_path.join(format!("{}.md", name));
        if !file_path.exists() {
            return Err(FileStorageError::PromptNotFound(
                file_path.display().to_string(),
            ));
        }

        fs::remove_file(file_path)?;
        Ok(())
    }
}

impl FileStorage {
    pub fn ensure_base_directory_exists(&self) -> Result<(), FileStorageError> {
        if !self.base_path.exists() {
            create_dir_all(&self.base_path)?;
        } else if !self.base_path.is_dir() {
            return Err(FileStorageError::InvalidBasePath(
                self.base_path.display().to_string(),
            ));
        }
        Ok(())
    }

    fn get_md_files(&self) -> Result<Vec<walkdir::DirEntry>, FileStorageError> {
        let entries = WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "md")
            })
            .collect();
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::Prompt;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_save_simple_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        let prompt = Prompt::new(
            PromptMetadata::new("test_prompt".to_string(), Some("A test prompt".to_string()), vec!["tag1".to_string(), "tag2".to_string()]),
            "This is a test prompt".to_string(),
        );

        let result = storage.save_prompt(&prompt);

        assert!(result.is_ok());

        // Check that the file was created
        let file_path = temp_dir.path().join("test_prompt.md");  // Save method creates .md files
        assert!(file_path.exists());

        // Check the content of the file
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("This is a test prompt"));
        assert!(content.contains("tag1"));
        assert!(content.contains("tag2"));
    }

    #[test]
    fn test_save_template_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        let metadata = PromptMetadata::new("template_prompt".to_string(), None, vec!["template".to_string()]);
        let prompt = Prompt::new(metadata, "This is a template prompt with {{variable}}".to_string());

        let result = storage.save_prompt(&prompt);

        assert!(result.is_ok());

        // Check that the file was created
        let file_path = temp_dir.path().join("template_prompt.md");
        assert!(file_path.exists());

        // Check the content of the file
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("This is a template prompt with {{variable}}"));
        assert!(content.contains("template"));
    }

    #[test]
    fn test_save_prompt_with_invalid_template_syntax() {
        // Create a prompt with invalid template syntax - this is allowed in the new architecture
        let metadata = PromptMetadata::new("invalid_template".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "This has invalid syntax {{unclosed".to_string());

        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Saving the prompt should work fine - storage doesn't validate template syntax
        let result = storage.save_prompt(&prompt);
        assert!(result.is_ok());

        // Only when we try to create a PromptTemplate do we get the error
        let loaded_result = storage.get_prompt("invalid_template");
        assert!(loaded_result.is_ok()); // Loading from storage should work
        
        // Creating a template from the loaded prompt will fail when parsing
        let template_result = PromptTemplate::new(loaded_result.unwrap());
        assert!(template_result.is_err());
        assert!(template_result
            .unwrap_err()
            .to_string()
            .contains("Parse template error"));
    }

    #[test]
    fn test_save_prompt_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let prompts_dir = temp_dir.path().join("prompts");
        let storage = FileStorage {
            base_path: prompts_dir.clone(),
        };

        // Directory should not exist yet
        assert!(!prompts_dir.exists());

        let metadata = PromptMetadata::new("dir_test".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "Test content".to_string());

        let result = storage.save_prompt(&prompt);

        assert!(result.is_ok());

        // Directory should now exist
        assert!(prompts_dir.exists());
        assert!(prompts_dir.is_dir());
    }

    #[test]
    fn test_save_prompt_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save first version
        let metadata1 = PromptMetadata::new("overwrite_test".to_string(), None, vec!["v1".to_string()]);
        let prompt1 = Prompt::new(metadata1, "First version".to_string());
        let result1 = storage.save_prompt(&prompt1);
        assert!(result1.is_ok());

        // Save second version (should overwrite)
        let metadata2 = PromptMetadata::new("overwrite_test".to_string(), None, vec!["v2".to_string()]);
        let prompt2 = Prompt::new(metadata2, "Second version".to_string());
        let result2 = storage.save_prompt(&prompt2);
        assert!(result2.is_ok());

        // Check that the file contains the second version
        let file_path = temp_dir.path().join("overwrite_test.md");
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("Second version"));
        assert!(content.contains("v2"));
        // Should not contain first version content
        assert!(!content.contains("v1"));
    }

    #[test]
    fn test_save_complex_template_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        let metadata = PromptMetadata::new("complex_template".to_string(), None, vec!["complex".to_string(), "template".to_string()]);
        let prompt = Prompt::new(metadata, "Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal}}}}".to_string());

        let result = storage.save_prompt(&prompt);
        assert!(result.is_ok());

        let file_path = temp_dir.path().join("complex_template.md");
        assert!(file_path.exists());

        let content = fs::read_to_string(file_path).unwrap();
        assert!(
            content.contains("Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal}}}}")
        );
        assert!(content.contains("complex"));
        assert!(content.contains("template"));
    }

    #[test]
    fn test_ensure_base_directory_exists_when_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("not_a_directory");

        // Create a file where we expect a directory
        fs::write(&file_path, "some content").unwrap();

        let storage = FileStorage {
            base_path: file_path,
        };

        let metadata = PromptMetadata::new("test".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "content".to_string());

        let result = storage.save_prompt(&prompt);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_simple_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // First save a simple prompt
        let metadata = PromptMetadata::new("load_test_simple".to_string(), None, vec!["test".to_string(), "simple".to_string()]);
        let original_prompt = Prompt::new(metadata, "This is a simple prompt for loading".to_string());
        storage.save_prompt(&original_prompt).unwrap();

        // Now load it back
        let result = storage.get_prompt("load_test_simple");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap();
        assert_eq!(loaded_prompt.metadata.name, "load_test_simple");
        assert_eq!(loaded_prompt.content, "This is a simple prompt for loading");
        assert_eq!(
            loaded_prompt.metadata.tags,
            vec!["test".to_string(), "simple".to_string()]
        );
    }

    #[test]
    fn test_load_template_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // First save a template prompt
        let metadata = PromptMetadata::new("load_test_template".to_string(), None, vec!["test".to_string(), "template".to_string()]);
        let original_prompt = Prompt::new(metadata, "Hello {{name}}, this is {{topic}}".to_string());
        storage.save_prompt(&original_prompt).unwrap();

        // Now load it back
        let result = storage.get_prompt("load_test_template");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap();
        assert_eq!(loaded_prompt.metadata.name, "load_test_template");
        assert_eq!(loaded_prompt.content, "Hello {{name}}, this is {{topic}}");
        assert_eq!(
            loaded_prompt.metadata.tags,
            vec!["test".to_string(), "template".to_string()]
        );
    }

    #[test]
    fn test_load_prompt_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        let result = storage.get_prompt("nonexistent_prompt");
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::PromptNotFound(path) => {
                assert!(path.contains("nonexistent_prompt.md"));
            }
            _ => panic!("Expected PromptNotFound error"),
        }
    }

    #[test]
    fn test_load_prompt_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create a file with invalid content
        let file_path = temp_dir.path().join("invalid.md");
        fs::write(file_path, "invalid content [[[").unwrap();

        let result = storage.get_prompt("invalid");
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::SerializationError(_) => {}
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_load_prompt_invalid_prompt_type() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create an invalid file
        let invalid_template_content: &str = r#"---
na_me: "invalid_template"
tags: ["example", "frontmatter", "rust"]
---

{{Hello world!"#;
        let file_path = temp_dir.path().join("invalid_type_test.md");
        fs::write(file_path, invalid_template_content).unwrap();

        let result = storage.get_prompt("invalid_type_test");
        assert!(result.is_err());
    }

    #[test]
    fn test_load_prompt_invalid_template_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create a MD file with proper YAML frontmatter but invalid template syntax in content
        let invalid_template_content: &str = r#"---
name: "invalid_template_syntax"
tags: ["example", "frontmatter", "rust"]
created: "2025-09-25T10:30:00Z"
last_modified: "2025-09-25T10:30:00Z"
---

{{Hello world!"#;
        let file_path = temp_dir.path().join("invalid_template_syntax.md");
        fs::write(file_path, invalid_template_content).unwrap();

        // Loading the prompt from storage should work since it doesn't validate template syntax
        let result = storage.get_prompt("invalid_template_syntax");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap();
        assert_eq!(loaded_prompt.metadata.name, "invalid_template_syntax");
        assert_eq!(loaded_prompt.content, "{{Hello world!");
    }

    #[test]
    fn test_load_prompt_missing_fields() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create a MD file with incomplete YAML frontmatter
        let incomplete_md = r#"---
name: "incomplete_test"
# missing other required fields like tags
---

Prompt content here"#;
        let file_path = temp_dir.path().join("incomplete_test.md");
        fs::write(file_path, incomplete_md).unwrap();

        let result = storage.get_prompt("incomplete_test");
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::SerializationError(_) => {}
            _ => panic!("Expected SerializationError for missing fields"),
        }
    }

    #[test]
    fn test_load_prompt_empty_tags() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a prompt with no tags
        let metadata = PromptMetadata::new("no_tags_test".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "Content without tags".to_string());
        storage.save_prompt(&prompt).unwrap();

        // Load it back
        let result = storage.get_prompt("no_tags_test");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap();
        assert_eq!(loaded_prompt.metadata.name, "no_tags_test");
        assert_eq!(loaded_prompt.content, "Content without tags");
        assert!(loaded_prompt.metadata.tags.is_empty());
    }

    #[test]
    fn test_load_prompt_complex_template() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a complex template prompt
        let metadata = PromptMetadata::new("complex_template_load".to_string(), None, vec![
            "complex".to_string(),
            "template".to_string(),
            "test".to_string(),
        ]);
        let complex_content =
            "Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal}}}} Today is {{date}}.";
        let original_prompt = Prompt::new(metadata, complex_content.to_string());
        storage.save_prompt(&original_prompt).unwrap();

        // Load it back
        let result = storage.get_prompt("complex_template_load");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap();
        assert_eq!(loaded_prompt.metadata.name, "complex_template_load");
        assert_eq!(loaded_prompt.content, complex_content);
        assert_eq!(
            loaded_prompt.metadata.tags,
            vec![
                "complex".to_string(),
                "template".to_string(),
                "test".to_string()
            ]
        );
    }

    #[test]
    fn test_load_prompt_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a prompt with special characters
        let special_content = "Content with special chars: ñáéíóú, 中文, emoji 🚀, quotes \"'`";
        let metadata = PromptMetadata::new("special_chars_test".to_string(), None, vec!["special".to_string(), "unicode".to_string()]);
        let original_prompt = Prompt::new(metadata, special_content.to_string());
        storage.save_prompt(&original_prompt).unwrap();

        // Load it back
        let result = storage.get_prompt("special_chars_test");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap();
        assert_eq!(loaded_prompt.content, special_content);
    }

    #[test]
    fn test_delete_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a prompt
        let metadata = PromptMetadata::new("delete_test".to_string(), None, vec!["test".to_string(), "delete".to_string()]);
        let prompt = Prompt::new(metadata, "This is a test prompt for deletion".to_string());
        storage.save_prompt(&prompt).unwrap();

        // Verify the file exists
        let file_path = temp_dir.path().join("delete_test.md");
        assert!(file_path.exists());

        // Delete the prompt
        let result = storage.delete_prompt("delete_test");
        assert!(result.is_ok());

        // Verify the file no longer exists
        assert!(!file_path.exists());

        // Try to delete a non-existent prompt (should fail)
        let result = storage.delete_prompt("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_prompts() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a few different prompts
        let simple_metadata = PromptMetadata::new("simple_test".to_string(), None, vec!["simple".to_string(), "test".to_string()]);
        let simple_prompt = Prompt::new(simple_metadata, "This is a simple prompt".to_string());
        storage.save_prompt(&simple_prompt).unwrap();

        let template_metadata = PromptMetadata::new("template_test".to_string(), None, vec!["template".to_string(), "test".to_string()]);
        let template_prompt = Prompt::new(template_metadata, "Hello {{name}}, welcome to {{prompt:greeting}}!".to_string());
        storage.save_prompt(&template_prompt).unwrap();

        // Get all prompts
        let result = storage.get_prompts();
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 2);

        // Find and verify each prompt
        let simple_found = prompts.iter().find(|p| p.metadata.name == "simple_test").unwrap();
        assert_eq!(simple_found.content, "This is a simple prompt");
        assert_eq!(
            simple_found.metadata.tags,
            vec!["simple".to_string(), "test".to_string()]
        );

        let template_found = prompts.iter().find(|p| p.metadata.name == "template_test").unwrap();
        assert_eq!(
            template_found.content,
            "Hello {{name}}, welcome to {{prompt:greeting}}!"
        );
        assert_eq!(
            template_found.metadata.tags,
            vec!["template".to_string(), "test".to_string()]
        );
    }

    #[test]
    fn test_get_prompts_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Get prompts from empty directory
        let result = storage.get_prompts();
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 0);
    }

    #[test]
    fn test_get_prompts_with_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create an invalid file
        let invalid_file_path = temp_dir.path().join("invalid.md");
        fs::write(invalid_file_path, "invalid content [[[").unwrap();

        // Get prompts - should fail due to invalid content
        let result = storage.get_prompts();
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::SerializationError(_) => {}
            _ => panic!("Expected SerializationError"),
        }
    }

    #[test]
    fn test_get_prompts_by_tag() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a few different prompts with different tags
        let simple_metadata = PromptMetadata::new("simple_test".to_string(), None, vec!["simple".to_string(), "test".to_string()]);
        let simple_prompt = Prompt::new(simple_metadata, "This is a simple prompt".to_string());
        storage.save_prompt(&simple_prompt).unwrap();

        let template_metadata = PromptMetadata::new("template_test".to_string(), None, vec!["template".to_string(), "test".to_string()]);
        let template_prompt = Prompt::new(template_metadata, "Hello {{name}}, welcome to {{prompt:greeting}}!".to_string());
        storage.save_prompt(&template_prompt).unwrap();

        let another_metadata = PromptMetadata::new("another_test".to_string(), None, vec!["another".to_string()]);
        let another_prompt = Prompt::new(another_metadata, "This is another prompt".to_string());
        storage.save_prompt(&another_prompt).unwrap();

        // Get prompts by "test" tag (should return 2 prompts)
        let result = storage.get_prompts_by_tag(&["test".to_string()]);
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 2);

        // Find and verify each prompt
        let simple_found = prompts.iter().find(|p| p.metadata.name == "simple_test").unwrap();
        assert_eq!(simple_found.content, "This is a simple prompt");
        assert_eq!(
            simple_found.metadata.tags,
            vec!["simple".to_string(), "test".to_string()]
        );

        let template_found = prompts.iter().find(|p| p.metadata.name == "template_test").unwrap();
        assert_eq!(
            template_found.content,
            "Hello {{name}}, welcome to {{prompt:greeting}}!"
        );
        assert_eq!(
            template_found.metadata.tags,
            vec!["template".to_string(), "test".to_string()]
        );

        // Get prompts by "another" tag (should return 1 prompt)
        let result = storage.get_prompts_by_tag(&["another".to_string()]);
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 1);

        let another_found = prompts.first().unwrap();
        assert_eq!(another_found.metadata.name, "another_test");
        assert_eq!(another_found.content, "This is another prompt");
        assert_eq!(another_found.metadata.tags, vec!["another".to_string()]);

        // Get prompts by a tag that doesn't exist (should return 0 prompts)
        let result = storage.get_prompts_by_tag(&["nonexistent".to_string()]);
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 0);

        // Get prompts by multiple tags (should return prompts matching any of the tags)
        let result = storage.get_prompts_by_tag(&["simple".to_string(), "another".to_string()]);
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 2);

        let simple_found = prompts.iter().find(|p| p.metadata.name == "simple_test").unwrap();
        let another_found = prompts.iter().find(|p| p.metadata.name == "another_test").unwrap();
        assert_eq!(simple_found.metadata.name, "simple_test");
        assert_eq!(another_found.metadata.name, "another_test");
    }

    #[test]
    fn test_get_prompts_by_tag_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Get prompts by tag from empty directory
        let result = storage.get_prompts_by_tag(&["test".to_string()]);
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 0);
    }

    #[test]
    fn test_get_prompts_by_tag_with_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create a valid prompt with a tag
        let metadata = PromptMetadata::new("valid_prompt".to_string(), None, vec!["valid".to_string()]);
        let prompt = Prompt::new(metadata, "This is a valid prompt".to_string());
        storage.save_prompt(&prompt).unwrap();

        // Create an invalid file
        let invalid_file_path = temp_dir.path().join("invalid.md");
        fs::write(invalid_file_path, "invalid content [[[[").unwrap();

        // Get prompts by tag - should fail due to invalid content
        let result = storage.get_prompts_by_tag(&["valid".to_string()]);
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::SerializationError(_) => {}
            _ => panic!("Expected SerializationError"),
        }
    }
}