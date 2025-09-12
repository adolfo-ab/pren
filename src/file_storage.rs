use std::error::Error;
use std::fs;
use std::fs::{create_dir_all};
use std::path::PathBuf;
use crate::registry::{PromptFile, PromptStorage};
use toml;

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
    fn save_prompt(&self, name: &str, content: &str, tags: Vec<String>, prompt_type: &str) -> Result<(), Box<dyn Error>> {
        if prompt_type != "simple" && prompt_type != "template" {
            return Err("Invalid prompt type. Must be 'simple' or 'template'.".into());
        }

        // Ensure base directory exists
        if !self.base_path.exists() {
            create_dir_all(&self.base_path)?;
        } else if !self.base_path.is_dir() {
            return Err("Base path is not a directory".into());
        }

        // Create file path with .toml extension
        let file_path = self.base_path.join(format!("{}.toml", name));

        let prompt_file = PromptFile {
            tags,
            name: name.to_string(),
            content: content.to_string(),
            prompt_type: prompt_type.to_string(),
        };

        let serialized_data = toml::to_string_pretty(&prompt_file)?;

        fs::write(&file_path, &serialized_data)?;

        Ok(())
    }

    fn load_prompt(&self, name: &str) -> Result<Option<PromptFile>, Box<dyn Error>> {
        todo!()
    }

    fn list_prompts(&self) -> Result<Vec<String>, Box<dyn Error>> {
        todo!()
    }

    fn delete_prompt(&self, name: &str) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn search_prompts_by_tags(&self, tags: &[String]) -> Result<Vec<PromptFile>, Box<dyn Error>> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_save_simple_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        let result = storage.save_prompt(
            "test_prompt",
            "This is a test prompt",
            vec!["tag1".to_string(), "tag2".to_string()],
            "simple"
        );

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

        let result = storage.save_prompt(
            "template_prompt",
            "This is a template prompt with {{variable}}",
            vec!["template".to_string()],
            "template"
        );

        assert!(result.is_ok());

        // Check that the file was created
        let file_path = temp_dir.path().join("template_prompt.toml");
        assert!(file_path.exists());

        // Check the content of the file
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("This is a template prompt with {{variable}}"));
        assert!(content.contains("template"));
        assert!(content.contains("template"));
    }

    #[test]
    fn test_save_prompt_invalid_type() {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileStorage {
            base_path: temp_dir.path().to_path_buf(),
        };

        let result = storage.save_prompt(
            "invalid_prompt",
            "This is an invalid prompt",
            vec![],
            "invalid"
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Invalid prompt type. Must be 'simple' or 'template'.");
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

        let result = storage.save_prompt(
            "dir_test",
            "Test content",
            vec![],
            "simple"
        );

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
        let result1 = storage.save_prompt(
            "overwrite_test",
            "First version",
            vec!["v1".to_string()],
            "simple"
        );
        assert!(result1.is_ok());

        // Save second version (should overwrite)
        let result2 = storage.save_prompt(
            "overwrite_test",
            "Second version",
            vec!["v2".to_string()],
            "simple"
        );
        assert!(result2.is_ok());

        // Check that the file contains the second version
        let file_path = temp_dir.path().join("overwrite_test.toml");
        let content = fs::read_to_string(file_path).unwrap();
        assert!(content.contains("Second version"));
        assert!(content.contains("v2"));
        // Should not contain first version content
        assert!(!content.contains("v1"));
    }
}