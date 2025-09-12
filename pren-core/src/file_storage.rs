use std::error::Error;
use std::{fmt, fs, io};
use std::fs::{create_dir_all};
use std::path::PathBuf;
use crate::registry::{PromptFile, PromptStorage};
use toml;
use walkdir::WalkDir;
use crate::prompt::{ParseTemplateError, Prompt};

#[derive(Debug)]
pub enum FileStorageError {
    IoError(io::Error),
    SerializationError(toml::ser::Error),
    DeserializationError(toml::de::Error),
    InvalidBasePath(String),
    PromptNotFound(String),
    InvalidPromptType(String),
    ParseTemplateError(ParseTemplateError),
}

impl fmt::Display for FileStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileStorageError::IoError(err) => write!(f, "IO error: {}", err),
            FileStorageError::SerializationError(err) => write!(f, "Failed to serialize prompt: {}", err),
            FileStorageError::DeserializationError(err) => write!(f, "Failed to deserialize prompt: {}", err),
            FileStorageError::InvalidBasePath(path) => write!(f, "Invalid base path: {}", path),
            FileStorageError::PromptNotFound(path) => write!(f, "Prompt not found: {}", path),
            FileStorageError::InvalidPromptType(prompt_type) => write!(f, "Invalid prompt type, must be 'simple' or 'template': {}", prompt_type),
            FileStorageError::ParseTemplateError(err) => write!(f, "{}", err),
        }
    }
}

impl Error for FileStorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FileStorageError::IoError(err) => Some(err),
            FileStorageError::SerializationError(err) => Some(err),
            FileStorageError::DeserializationError(err) => Some(err),
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

impl From<toml::ser::Error> for FileStorageError {
    fn from(err: toml::ser::Error) -> Self {
        FileStorageError::SerializationError(err)
    }
}

impl From<toml::de::Error> for FileStorageError {
    fn from(err: toml::de::Error) -> Self {
        FileStorageError::DeserializationError(err)
    }
}

impl From<ParseTemplateError> for FileStorageError {
    fn from(err: ParseTemplateError) -> Self {
        FileStorageError::ParseTemplateError(err)
    }
}

pub struct FileStorage {
    pub base_path: PathBuf,
}

impl Default for FileStorage {
    fn default() -> Self {
        Self {
            base_path: PathBuf::from("./prompts")
        }
    }
}

impl PromptStorage for FileStorage{
    fn save_prompt(&self, prompt: &Prompt) -> Result<(), FileStorageError> {
        self.ensure_base_directory_exists()?;

        let file_path = self.base_path.join(format!("{}.toml", prompt.name()));

        let prompt_file = PromptFile {
            tags: prompt.tags().clone(),
            name: prompt.name().to_string(),
            content: prompt.content().to_string(),
            prompt_type: match prompt {
                Prompt::Simple { .. } => "simple".to_string(),
                Prompt::Template { .. } => "template".to_string(),
            },
        };

        let serialized_data = toml::to_string_pretty(&prompt_file)?;
        fs::write(file_path, serialized_data)?;

        Ok(())
    }

    fn get_prompt(&self, name: &str) -> Result<Option<Prompt>, FileStorageError> {
        let file_path = self.base_path.join(format!("{}.toml", name));
        if !file_path.exists() {
            return Err(FileStorageError::PromptNotFound(
                file_path.display().to_string()
            ))
        }

        let content = fs::read_to_string(file_path)?;
        let prompt_file: PromptFile = toml::from_str(&content)?;

        let prompt = match prompt_file.prompt_type.as_str() {
            "simple" => Prompt::new_simple(prompt_file.name, prompt_file.content, prompt_file.tags),
            "template" => Prompt::new_template(prompt_file.name, prompt_file.content, prompt_file.tags)?,
            _ => return Err(FileStorageError::InvalidPromptType(prompt_file.prompt_type))
        };

        Ok(Some(prompt))
    }

    fn get_prompts(&self) -> Result<Vec<Prompt>, FileStorageError> {
        let mut prompts = Vec::new();
        
        // Walk through the base directory
        for entry in WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "toml"))
        {
            let file_path = entry.path();
            
            // Read and parse the file
            let content = fs::read_to_string(file_path)?;
            let prompt_file: PromptFile = toml::from_str(&content)?;
            
            // Convert to Prompt based on type
            let prompt = match prompt_file.prompt_type.as_str() {
                "simple" => Prompt::new_simple(prompt_file.name, prompt_file.content, prompt_file.tags),
                "template" => Prompt::new_template(prompt_file.name, prompt_file.content, prompt_file.tags)?,
                _ => return Err(FileStorageError::InvalidPromptType(prompt_file.prompt_type))
            };
            
            prompts.push(prompt);
        }
        
        Ok(prompts)
    }

    fn get_prompts_by_tag(&self, tags: &[String]) -> Result<Vec<Prompt>, FileStorageError> {
        let mut prompts = Vec::new();

        // Walk through the base directory
        for entry in WalkDir::new(&self.base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "toml"))
        {
            let file_path = entry.path();

            // Read and parse the file
            let content = fs::read_to_string(file_path)?;
            let prompt_file: PromptFile = toml::from_str(&content)?;

            // Check if any of the prompt's tags match any of the requested tags
            if prompt_file.tags.iter().any(|prompt_tag| tags.contains(prompt_tag)) {
                let prompt = match prompt_file.prompt_type.as_str() {
                    "simple" => Prompt::new_simple(prompt_file.name, prompt_file.content, prompt_file.tags),
                    "template" => Prompt::new_template(prompt_file.name, prompt_file.content, prompt_file.tags)?,
                    _ => return Err(FileStorageError::InvalidPromptType(prompt_file.prompt_type))
                };
                prompts.push(prompt);
            }
        }

        Ok(prompts)
    }

    fn delete_prompt(&self, name: &str) -> Result<(), FileStorageError> {
        let file_path = self.base_path.join(format!("{}.toml", name));
        if !file_path.exists() {
            return Ok(());
        }

        fs::remove_file(file_path)?;
        Ok(())
    }
}

impl FileStorage {
    fn ensure_base_directory_exists(&self) -> Result<(), FileStorageError> {
        if !self.base_path.exists() {
            create_dir_all(&self.base_path)?;
        } else if !self.base_path.is_dir() {
            return Err(FileStorageError::InvalidBasePath(
                self.base_path.display().to_string()
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use crate::prompt::Prompt;

    #[test]
    fn test_save_simple_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        let prompt = Prompt::new_simple(
            "test_prompt".to_string(),
            "This is a test prompt".to_string(),
            vec!["tag1".to_string(), "tag2".to_string()]
        );

        let result = storage.save_prompt(&prompt);

        assert!(result.is_ok());

        // Check that the file was created
        let file_path = temp_dir.path().join("test_prompt.toml");
        assert!(file_path.exists());

        // Check the content of the file
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("This is a test prompt"));
        assert!(content.contains("tag1"));
        assert!(content.contains("tag2"));
        assert!(content.contains("simple"));
    }

    #[test]
    fn test_save_template_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        let prompt = Prompt::new_template(
            "template_prompt".to_string(),
            "This is a template prompt with {{variable}}".to_string(),
            vec!["template".to_string()]
        ).expect("Failed to create template prompt");

        let result = storage.save_prompt(&prompt);

        assert!(result.is_ok());

        // Check that the file was created
        let file_path = temp_dir.path().join("template_prompt.toml");
        assert!(file_path.exists());

        // Check the content of the file
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("This is a template prompt with {{variable}}"));
        assert!(content.contains("template"));
    }

    #[test]
    fn test_save_prompt_invalid_template() {
        // Test that invalid template syntax fails at prompt creation time
        let result = Prompt::new_template(
            "invalid_template".to_string(),
            "This has invalid syntax {{unclosed".to_string(),
            vec![]
        );

        assert!(result.is_err());
        // The error should be a ParseTemplateError
        assert!(result.unwrap_err().to_string().contains("Parse template error"));
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

        let prompt = Prompt::new_simple(
            "dir_test".to_string(),
            "Test content".to_string(),
            vec![]
        );

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
        let prompt1 = Prompt::new_simple(
            "overwrite_test".to_string(),
            "First version".to_string(),
            vec!["v1".to_string()]
        );
        let result1 = storage.save_prompt(&prompt1);
        assert!(result1.is_ok());

        // Save second version (should overwrite)
        let prompt2 = Prompt::new_simple(
            "overwrite_test".to_string(),
            "Second version".to_string(),
            vec!["v2".to_string()]
        );
        let result2 = storage.save_prompt(&prompt2);
        assert!(result2.is_ok());

        // Check that the file contains the second version
        let file_path = temp_dir.path().join("overwrite_test.toml");
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

        let prompt = Prompt::new_template(
            "complex_template".to_string(),
            "Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal}}}}".to_string(),
            vec!["complex".to_string(), "template".to_string()]
        ).expect("Failed to create complex template");

        let result = storage.save_prompt(&prompt);
        assert!(result.is_ok());

        let file_path = temp_dir.path().join("complex_template.toml");
        assert!(file_path.exists());

        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal}}}}"));
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

        let prompt = Prompt::new_simple(
            "test".to_string(),
            "content".to_string(),
            vec![]
        );

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
        let original_prompt = Prompt::new_simple(
            "load_test_simple".to_string(),
            "This is a simple prompt for loading".to_string(),
            vec!["test".to_string(), "simple".to_string()]
        );
        storage.save_prompt(&original_prompt).unwrap();

        // Now load it back
        let result = storage.get_prompt("load_test_simple");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap().unwrap();
        assert_eq!(loaded_prompt.name(), "load_test_simple");
        assert_eq!(loaded_prompt.content(), "This is a simple prompt for loading");
        assert_eq!(loaded_prompt.tags(), &vec!["test".to_string(), "simple".to_string()]);

        // Verify it's a simple prompt
        match loaded_prompt {
            Prompt::Simple { .. } => {},
            _ => panic!("Expected Simple prompt variant"),
        }
    }

    #[test]
    fn test_load_template_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // First save a template prompt
        let original_prompt = Prompt::new_template(
            "load_test_template".to_string(),
            "Hello {{name}}, this is {{topic}}".to_string(),
            vec!["test".to_string(), "template".to_string()]
        ).unwrap();
        storage.save_prompt(&original_prompt).unwrap();

        // Now load it back
        let result = storage.get_prompt("load_test_template");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap().unwrap();
        assert_eq!(loaded_prompt.name(), "load_test_template");
        assert_eq!(loaded_prompt.content(), "Hello {{name}}, this is {{topic}}");
        assert_eq!(loaded_prompt.tags(), &vec!["test".to_string(), "template".to_string()]);

        // Verify it's a template prompt
        match loaded_prompt {
            Prompt::Template { .. } => {},
            _ => panic!("Expected Template prompt variant"),
        }
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
                assert!(path.contains("nonexistent_prompt.toml"));
            },
            _ => panic!("Expected PromptNotFound error"),
        }
    }

    #[test]
    fn test_load_prompt_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create a file with invalid TOML content
        let file_path = temp_dir.path().join("invalid_toml.toml");
        fs::write(file_path, "invalid toml content [[[").unwrap();

        let result = storage.get_prompt("invalid_toml");
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::DeserializationError(_) => {},
            _ => panic!("Expected DeserializationError"),
        }
    }

    #[test]
    fn test_load_prompt_invalid_prompt_type() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create a TOML file with invalid prompt_type
        let invalid_toml = r#"
            name = "invalid_type_test"
            content = "Some content"
            tags = ["test"]
            type = "invalid_type"
        "#;
        let file_path = temp_dir.path().join("invalid_type_test.toml");
        fs::write(file_path, invalid_toml).unwrap();

        let result = storage.get_prompt("invalid_type_test");
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::InvalidPromptType(prompt_type) => {
                assert_eq!(prompt_type, "invalid_type");
            },
            _ => panic!("Expected InvalidPromptType error"),
        }
    }

    #[test]
    fn test_load_prompt_invalid_template_syntax() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create a TOML file with template type but invalid template syntax
        let invalid_template_toml = r#"
            name = "invalid_template_syntax"
            content = "This has invalid syntax {{unclosed"
            tags = ["test"]
            type = "template"
        "#;
        let file_path = temp_dir.path().join("invalid_template_syntax.toml");
        fs::write(file_path, invalid_template_toml).unwrap();

        let result = storage.get_prompt("invalid_template_syntax");
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::ParseTemplateError(_) => {},
            _ => panic!("Expected ParseTemplateError"),
        }
    }

    #[test]
    fn test_load_prompt_missing_fields() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Create a TOML file missing required fields
        let incomplete_toml = r#"
            name = "incomplete_test"
            # missing content, tags, and prompt_type
        "#;
        let file_path = temp_dir.path().join("incomplete_test.toml");
        fs::write(file_path, incomplete_toml).unwrap();

        let result = storage.get_prompt("incomplete_test");
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::DeserializationError(_) => {},
            _ => panic!("Expected DeserializationError for missing fields"),
        }
    }

    #[test]
    fn test_load_prompt_empty_tags() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a prompt with no tags
        let prompt = Prompt::new_simple(
            "no_tags_test".to_string(),
            "Content without tags".to_string(),
            vec![]
        );
        storage.save_prompt(&prompt).unwrap();

        // Load it back
        let result = storage.get_prompt("no_tags_test");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap().unwrap();
        assert_eq!(loaded_prompt.name(), "no_tags_test");
        assert_eq!(loaded_prompt.content(), "Content without tags");
        assert!(loaded_prompt.tags().is_empty());
    }

    #[test]
    fn test_load_prompt_complex_template() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a complex template prompt
        let complex_content = "Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal}}}} Today is {{date}}.";
        let original_prompt = Prompt::new_template(
            "complex_template_load".to_string(),
            complex_content.to_string(),
            vec!["complex".to_string(), "template".to_string(), "test".to_string()]
        ).unwrap();
        storage.save_prompt(&original_prompt).unwrap();

        // Load it back
        let result = storage.get_prompt("complex_template_load");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap().unwrap();
        assert_eq!(loaded_prompt.name(), "complex_template_load");
        assert_eq!(loaded_prompt.content(), complex_content);
        assert_eq!(loaded_prompt.tags(), &vec!["complex".to_string(), "template".to_string(), "test".to_string()]);

        // Verify it's a template prompt
        match loaded_prompt {
            Prompt::Template { .. } => {},
            _ => panic!("Expected Template prompt variant"),
        }
    }

    #[test]
    fn test_load_prompt_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a prompt with special characters
        let special_content = "Content with special chars: ñáéíóú, 中文, emoji 🚀, quotes \"'`";
        let original_prompt = Prompt::new_simple(
            "special_chars_test".to_string(),
            special_content.to_string(),
            vec!["special".to_string(), "unicode".to_string()]
        );
        storage.save_prompt(&original_prompt).unwrap();

        // Load it back
        let result = storage.get_prompt("special_chars_test");
        assert!(result.is_ok());

        let loaded_prompt = result.unwrap().unwrap();
        assert_eq!(loaded_prompt.content(), special_content);
    }

    #[test]
    fn test_delete_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a prompt
        let prompt = Prompt::new_simple(
            "delete_test".to_string(),
            "This is a test prompt for deletion".to_string(),
            vec!["test".to_string(), "delete".to_string()]
        );
        storage.save_prompt(&prompt).unwrap();

        // Verify the file exists
        let file_path = temp_dir.path().join("delete_test.toml");
        assert!(file_path.exists());

        // Delete the prompt
        let result = storage.delete_prompt("delete_test");
        assert!(result.is_ok());

        // Verify the file no longer exists
        assert!(!file_path.exists());

        // Try to delete a non-existent prompt (should succeed)
        let result = storage.delete_prompt("nonexistent");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_prompts() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a few different prompts
        let simple_prompt = Prompt::new_simple(
            "simple_test".to_string(),
            "This is a simple prompt".to_string(),
            vec!["simple".to_string(), "test".to_string()]
        );
        storage.save_prompt(&simple_prompt).unwrap();

        let template_prompt = Prompt::new_template(
            "template_test".to_string(),
            "Hello {{name}}, welcome to {{prompt:greeting}}!".to_string(),
            vec!["template".to_string(), "test".to_string()]
        ).unwrap();
        storage.save_prompt(&template_prompt).unwrap();

        // Get all prompts
        let result = storage.get_prompts();
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 2);

        // Find and verify each prompt
        let simple_found = prompts.iter().find(|p| p.name() == "simple_test").unwrap();
        assert_eq!(simple_found.content(), "This is a simple prompt");
        assert_eq!(simple_found.tags(), &vec!["simple".to_string(), "test".to_string()]);
        match simple_found {
            Prompt::Simple { .. } => {},
            _ => panic!("Expected Simple prompt variant"),
        }

        let template_found = prompts.iter().find(|p| p.name() == "template_test").unwrap();
        assert_eq!(template_found.content(), "Hello {{name}}, welcome to {{prompt:greeting}}!");
        assert_eq!(template_found.tags(), &vec!["template".to_string(), "test".to_string()]);
        match template_found {
            Prompt::Template { .. } => {},
            _ => panic!("Expected Template prompt variant"),
        }
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

        // Create a valid prompt file
        let prompt = Prompt::new_simple(
            "valid_prompt".to_string(),
            "This is a valid prompt".to_string(),
            vec!["valid".to_string()]
        );
        storage.save_prompt(&prompt).unwrap();

        // Create an invalid TOML file
        let invalid_file_path = temp_dir.path().join("invalid.toml");
        fs::write(invalid_file_path, "invalid toml content [[[").unwrap();

        // Get prompts - should fail due to invalid TOML
        let result = storage.get_prompts();
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::DeserializationError(_) => {},
            _ => panic!("Expected DeserializationError"),
        }
    }

    #[test]
    fn test_get_prompts_by_tag() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        // Save a few different prompts with different tags
        let simple_prompt = Prompt::new_simple(
            "simple_test".to_string(),
            "This is a simple prompt".to_string(),
            vec!["simple".to_string(), "test".to_string()]
        );
        storage.save_prompt(&simple_prompt).unwrap();

        let template_prompt = Prompt::new_template(
            "template_test".to_string(),
            "Hello {{name}}, welcome to {{prompt:greeting}}!".to_string(),
            vec!["template".to_string(), "test".to_string()]
        ).unwrap();
        storage.save_prompt(&template_prompt).unwrap();

        let another_prompt = Prompt::new_simple(
            "another_test".to_string(),
            "This is another prompt".to_string(),
            vec!["another".to_string()]
        );
        storage.save_prompt(&another_prompt).unwrap();

        // Get prompts by "test" tag (should return 2 prompts)
        let result = storage.get_prompts_by_tag(&["test".to_string()]);
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 2);

        // Find and verify each prompt
        let simple_found = prompts.iter().find(|p| p.name() == "simple_test").unwrap();
        assert_eq!(simple_found.content(), "This is a simple prompt");
        assert_eq!(simple_found.tags(), &vec!["simple".to_string(), "test".to_string()]);
        match simple_found {
            Prompt::Simple { .. } => {},
            _ => panic!("Expected Simple prompt variant"),
        }

        let template_found = prompts.iter().find(|p| p.name() == "template_test").unwrap();
        assert_eq!(template_found.content(), "Hello {{name}}, welcome to {{prompt:greeting}}!");
        assert_eq!(template_found.tags(), &vec!["template".to_string(), "test".to_string()]);
        match template_found {
            Prompt::Template { .. } => {},
            _ => panic!("Expected Template prompt variant"),
        }

        // Get prompts by "another" tag (should return 1 prompt)
        let result = storage.get_prompts_by_tag(&["another".to_string()]);
        assert!(result.is_ok());

        let prompts = result.unwrap();
        assert_eq!(prompts.len(), 1);

        let another_found = prompts.first().unwrap();
        assert_eq!(another_found.name(), "another_test");
        assert_eq!(another_found.content(), "This is another prompt");
        assert_eq!(another_found.tags(), &vec!["another".to_string()]);

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

        let simple_found = prompts.iter().find(|p| p.name() == "simple_test").unwrap();
        let another_found = prompts.iter().find(|p| p.name() == "another_test").unwrap();
        assert_eq!(simple_found.name(), "simple_test");
        assert_eq!(another_found.name(), "another_test");
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
        let prompt = Prompt::new_simple(
            "valid_prompt".to_string(),
            "This is a valid prompt".to_string(),
            vec!["valid".to_string()]
        );
        storage.save_prompt(&prompt).unwrap();

        // Create an invalid TOML file
        let invalid_file_path = temp_dir.path().join("invalid.toml");
        fs::write(invalid_file_path, "invalid toml content [[[[").unwrap();

        // Get prompts by tag - should fail due to invalid TOML
        let result = storage.get_prompts_by_tag(&["valid".to_string()]);
        assert!(result.is_err());

        match result.unwrap_err() {
            FileStorageError::DeserializationError(_) => {},
            _ => panic!("Expected DeserializationError"),
        }
    }
}