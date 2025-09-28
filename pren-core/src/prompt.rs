//! # Prompt Management
//!
//! This module provides core functionality for managing prompts, including both simple prompts
//! and template-based prompts with variable substitution.
//!
//! # Examples
//!
//! Creating a simple prompt:
//!
//! ```rust
//! use pren_core::prompt::{Prompt, PromptMetadata};
//!
//! let metadata = PromptMetadata::new("greeting".to_string(), None, vec!["example".to_string()]);
//! let prompt = Prompt::new(metadata, "Hello, world!".to_string());
//! ```
//!
//! Creating a template prompt:
//!
//! ```rust
//! use pren_core::prompt::{Prompt, PromptMetadata};
//!
//! let metadata = PromptMetadata::new("personal_greeting".to_string(), None, vec!["example".to_string()]);
//! let prompt = Prompt::new(metadata, "Hello {{name}}, welcome to {{prompt:service_name}}!".to_string());
//! ```

use crate::parser::parse_template;
use crate::storage::PromptStorage;
use nom::Err as NomErr;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use thiserror::Error;


/// Maximum allowed nesting depth for prompt templates
const MAX_NESTING_DEPTH: usize = 3; // TODO: Make this a variable

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMetadata {
    /// The name of the prompt.
    pub name: String,
    /// A brief description for the prompt.
    pub description: Option<String>,
    /// Tags used for searching.
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Prompt {
    pub metadata: PromptMetadata,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptTemplatePart {
    /// Literal text that is rendered as-is.
    Literal(String),
    /// An argument placeholder that gets replaced with a value at render time.
    Argument(String),
    /// A reference to another prompt that gets rendered at render time.
    PromptReference(String),
    /// A variable reference to another prompt that gets rendered at render time.
    VariablePromptReference(String),
}

/// A parsed template with parts that can be literals, arguments, or prompt references.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    // The prompt used to generate the template
    pub prompt: Prompt,
    /// The parts that make up the template.
    pub parts: Vec<PromptTemplatePart>,
}

#[derive(Error, Debug)]
#[error("Error found while parsing template: {message}")]
pub struct ParseTemplateError {
    pub message: String,
}

#[derive(Error, Debug)]
#[error("Error found while rendering template: {message}")]
pub struct RenderTemplateError {
    pub message: String,
}

/// A context for validating prompt templates during rendering, tracking visited prompts and current depth
#[derive(Debug, Clone)]
struct RenderValidationContext {
    /// The names of prompts visited in the current rendering path (to detect circular references)
    visited_prompts: HashSet<String>,
    /// The current nesting depth
    current_depth: usize,
}

impl RenderValidationContext {
    fn new() -> Self {
        RenderValidationContext {
            visited_prompts: HashSet::new(),
            current_depth: 0,
        }
    }

    fn enter_prompt(&mut self, prompt_name: &str) -> Result<(), RenderTemplateError> {
        // Check for circular references
        if self.visited_prompts.contains(prompt_name) {
            return Err(RenderTemplateError {
                message: format!(
                    "Circular reference detected: prompt '{}' references itself (directly or indirectly)",
                    prompt_name
                ),
            });
        }

        // Check depth limit
        if self.current_depth >= MAX_NESTING_DEPTH {
            return Err(RenderTemplateError {
                message: format!("Maximum nesting depth of {} exceeded", MAX_NESTING_DEPTH),
            });
        }

        self.visited_prompts.insert(prompt_name.to_string());
        self.current_depth += 1;
        Ok(())
    }

    fn exit_prompt(&mut self, prompt_name: &str) {
        self.visited_prompts.remove(prompt_name);
        self.current_depth -= 1;
    }
}

impl PromptMetadata {
    pub fn new(name: String, description: Option<String>, tags: Vec<String>) -> PromptMetadata {
        PromptMetadata {
            name,
            description,
            tags,
        }
    }
}

impl Prompt {
    pub fn new(metadata: PromptMetadata, content: String) -> Prompt {
        Prompt { metadata, content }
    }
}

impl PromptTemplate {
    /// Creates a new prompt template.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the prompt.
    /// * `content` - The content of the prompt with template syntax.
    /// * `template` - The PromptTemplate resulting of parsing the content.
    /// * `tags` - A vector of tags associated with the prompt.
    ///
    /// # Returns
    ///
    /// * `Ok(Prompt)` - A new `Prompt::Template` variant.
    /// * `Err(ParseTemplateError)` - If the template syntax is invalid.
    pub fn new(prompt: Prompt) -> Result<PromptTemplate, ParseTemplateError> {
        match parse_template(&prompt.content) {
            Ok((_, template_parts)) => Ok(PromptTemplate {
                prompt,
                parts: template_parts,
            }),
            Err(NomErr::Error(e)) | Err(NomErr::Failure(e)) => Err(ParseTemplateError {
                message: format!("Failed to parse template: {:?}", e),
            }),
            Err(NomErr::Incomplete(_)) => Err(ParseTemplateError {
                message: "Failed to parse template: incomplete input".to_string(),
            }),
        }
    }

    pub fn arguments(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter_map(|part| {
                if let PromptTemplatePart::Argument(arg) = part {
                    Some(arg.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn prompt_references(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter_map(|part| {
                if let PromptTemplatePart::PromptReference(prompt) = part {
                    Some(prompt.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn variable_prompt_references(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter_map(|part| {
                if let PromptTemplatePart::VariablePromptReference(prompt) = part {
                    Some(prompt.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn is_simple(&self) -> bool {
        self.arguments().len() == 0
            && self.prompt_references().len() == 0
            && self.variable_prompt_references().len() == 0
    }

    pub fn render<S: PromptStorage>(
        &self,
        arguments: &HashMap<String, String>,
        storage: &S,
    ) -> Result<String, RenderTemplateError> {
        let mut context = RenderValidationContext::new();
        self.render_internal(arguments, storage, &mut context)
    }

    /// Internal rendering function with validation context
    fn render_internal<S: PromptStorage>(
        &self,
        arguments: &HashMap<String, String>,
        storage: &S,
        context: &mut RenderValidationContext,
    ) -> Result<String, RenderTemplateError> {
        let mut result = String::new();

        for part in &self.parts {
            match part {
                PromptTemplatePart::Literal(text) => result.push_str(text),
                PromptTemplatePart::Argument(name) => match arguments.get(name) {
                    Some(value) => result.push_str(value),
                    None => {
                        return Err(RenderTemplateError {
                            message: format!("Missing argument: {}", name),
                        });
                    }
                },
                PromptTemplatePart::PromptReference(name) => {
                    self.render_prompt_reference(
                        name,
                        arguments,
                        storage,
                        context,
                        &mut result,
                        false,
                    )?;
                }
                PromptTemplatePart::VariablePromptReference(name) => match arguments.get(name) {
                    Some(value) => {
                        self.render_prompt_reference(
                            value,
                            arguments,
                            storage,
                            context,
                            &mut result,
                            true,
                        )?;
                    }
                    None => {
                        return Err(RenderTemplateError {
                            message: format!("Missing argument: {}", name),
                        });
                    }
                },
            }
        }
        Ok(result)
    }

    /// Helper function to render a prompt reference
    fn render_prompt_reference<S: PromptStorage>(
        &self,
        prompt_name: &str,
        arguments: &HashMap<String, String>,
        storage: &S,
        context: &mut RenderValidationContext,
        result: &mut String,
        is_variable_reference: bool,
    ) -> Result<(), RenderTemplateError> {
        // Validate before resolving the prompt reference
        context.enter_prompt(prompt_name)?;

        match storage.get_prompt(prompt_name) {
            Ok(prompt) => match PromptTemplate::new(prompt) {
                Ok(template) => match template.render_internal(arguments, storage, context) {
                    Ok(rendered) => result.push_str(&rendered),
                    Err(e) => {
                        context.exit_prompt(prompt_name);
                        return Err(RenderTemplateError {
                            message: format!(
                                "Failed to render referenced prompt '{}': {}",
                                prompt_name, e.message
                            ),
                        });
                    }
                },
                Err(e) => {
                    context.exit_prompt(prompt_name);
                    return Err(RenderTemplateError {
                        message: format!(
                            "Error parsing referenced prompt '{}': {}",
                            prompt_name, e
                        ),
                    });
                }
            },
            Err(e) => {
                context.exit_prompt(prompt_name);
                return Err(RenderTemplateError {
                    message: format!(
                        "Error retrieving referenced prompt '{}': {}",
                        prompt_name, e
                    ),
                });
            }
        }

        // Exit the prompt after successful rendering
        // For variable references, the caller is responsible for exiting
        if !is_variable_reference {
            context.exit_prompt(prompt_name);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::PromptStorage;

    #[test]
    fn test_new_simple_prompt() {
        let name = "prompt_name";
        let description = Some("A simple description".to_string());
        let content = "This is the prompt content";
        let tags = vec!["tag1".to_string(), "tag2".to_string()];

        let metadata = PromptMetadata::new(name.to_string(), description, tags.clone());
        let prompt = Prompt::new(metadata, content.to_string());

        let result = PromptTemplate::new(prompt);

        assert!(result.is_ok());

        let prompt_template = result.unwrap();

        assert_eq!(name, prompt_template.prompt.metadata.name);
        assert_eq!(content, prompt_template.prompt.content);
        assert_eq!(1, prompt_template.parts.len());
        assert_eq!(2, prompt_template.prompt.metadata.tags.len());
        assert_eq!(tags[0], prompt_template.prompt.metadata.tags[0]);
        assert_eq!(tags[1], prompt_template.prompt.metadata.tags[1]);
    }

    #[test]
    fn test_new_template_prompt() {
        let name = "complex_prompt";
        let content = "Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal_braces}}}}";
        let tags = vec!["tag1".to_string(), "tag2".to_string()];

        let metadata = PromptMetadata::new(name.to_string(), None, tags.clone());
        let prompt = Prompt::new(metadata, content.to_string());

        let result = PromptTemplate::new(prompt);
        assert!(result.is_ok());

        let template = result.unwrap();
        assert_eq!(name, template.prompt.metadata.name);
        assert_eq!(content, template.prompt.content);

        assert_eq!(6, template.parts.len());

        // Check each part
        match &template.parts[0] {
            PromptTemplatePart::Literal(text) => assert_eq!("Hello ", text),
            _ => panic!("Expected Literal part"),
        }

        match &template.parts[1] {
            PromptTemplatePart::Argument(arg) => assert_eq!("name", arg),
            _ => panic!("Expected Argument part"),
        }

        match &template.parts[2] {
            PromptTemplatePart::Literal(text) => assert_eq!(", welcome to ", text),
            _ => panic!("Expected Literal part"),
        }

        match &template.parts[3] {
            PromptTemplatePart::PromptReference(prompt_name) => {
                assert_eq!("greeting", prompt_name)
            }
            _ => panic!("Expected PromptReference part"),
        }

        match &template.parts[4] {
            PromptTemplatePart::Literal(text) => assert_eq!("! ", text),
            _ => panic!("Expected Literal part"),
        }

        match &template.parts[5] {
            PromptTemplatePart::Literal(text) => assert_eq!("literal_braces", text),
            _ => panic!("Expected Literal part"),
        }
    }

    struct MockStorage {
        prompts: HashMap<String, Prompt>,
    }

    impl MockStorage {
        fn new() -> Self {
            MockStorage {
                prompts: HashMap::new(),
            }
        }

        fn add_prompt(&mut self, prompt: Prompt) {
            self.prompts.insert(prompt.metadata.name.clone(), prompt);
        }
    }

    use std::error::Error;

    #[derive(Debug)]
    pub struct MockStorageError {
        message: String,
    }

    impl std::fmt::Display for MockStorageError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl Error for MockStorageError {}

    impl From<String> for MockStorageError {
        fn from(message: String) -> Self {
            MockStorageError { message }
        }
    }

    impl PromptStorage for MockStorage {
        type Error = MockStorageError;

        fn save_prompt(&self, _prompt: &Prompt) -> Result<(), Self::Error> {
            Ok(())
        }

        fn get_prompt(&self, name: &str) -> Result<Prompt, Self::Error> {
            match self.prompts.get(name) {
                Some(prompt) => Ok(prompt.clone()),
                None => Err("Prompt not found".to_string().into()),
            }
        }

        fn get_prompts(&self) -> Result<Vec<Prompt>, Self::Error> {
            Ok(self.prompts.values().cloned().collect())
        }

        fn get_prompts_by_tag(&self, _tags: &[String]) -> Result<Vec<Prompt>, Self::Error> {
            Ok(vec![])
        }

        fn delete_prompt(&self, _name: &str) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn test_render_simple_prompt() {
        let metadata = PromptMetadata::new("simple".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "This is a simple prompt".to_string());
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "World".to_string());

        let storage = MockStorage::new();
        let rendered = template
            .render(&args, &storage)
            .expect("Failed to render simple prompt");
        assert_eq!("This is a simple prompt", rendered);
    }

    #[test]
    fn test_render_template_prompt() {
        let metadata = PromptMetadata::new("template".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "Hello {{name}}, welcome!".to_string());
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "World".to_string());

        let storage = MockStorage::new();
        let rendered = template
            .render(&args, &storage)
            .expect("Failed to render template prompt");
        assert_eq!("Hello World, welcome!", rendered);
    }

    #[test]
    fn test_render_template_prompt_missing_argument() {
        let metadata = PromptMetadata::new("template".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "Hello {{name}}, welcome!".to_string());
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let args = HashMap::new();

        let storage = MockStorage::new();
        let result = template.render(&args, &storage);
        assert!(result.is_err());
        assert_eq!("Missing argument: name", result.unwrap_err().message);
    }

    #[test]
    fn test_render_template_prompt_multiple_arguments() {
        let metadata = PromptMetadata::new("template".to_string(), None, vec![]);
        let prompt = Prompt::new(
            metadata,
            "Dear {{name}}, you are {{age}} years old!".to_string(),
        );
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        args.insert("age".to_string(), "30".to_string());

        let storage = MockStorage::new();
        let rendered = template
            .render(&args, &storage)
            .expect("Failed to render template prompt");
        assert_eq!("Dear Alice, you are 30 years old!", rendered);
    }

    #[test]
    fn test_render_template_prompt_with_escaped_literals() {
        let metadata = PromptMetadata::new("template".to_string(), None, vec![]);
        let prompt = Prompt::new(
            metadata,
            "Hello {{{{{{name}}}}}}, you are {{age}} years old!".to_string(),
        );
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let mut args = HashMap::new();
        args.insert("age".to_string(), "30".to_string());

        let storage = MockStorage::new();
        let rendered = template
            .render(&args, &storage)
            .expect("Failed to render template prompt");
        assert_eq!("Hello {{name}}, you are 30 years old!", rendered);
    }

    #[test]
    fn test_render_template_with_prompt_reference() {
        let greeting_metadata = PromptMetadata::new("greeting".to_string(), None, vec![]);
        let greeting_prompt = Prompt::new(greeting_metadata, "Hello!".to_string());

        let main_metadata = PromptMetadata::new("main".to_string(), None, vec![]);
        let main_prompt = Prompt::new(
            main_metadata,
            "{{prompt:greeting}} Nice to meet you {{name}}!".to_string(),
        );
        let main_template = PromptTemplate::new(main_prompt).expect("Failed to create template");

        let mut storage = MockStorage::new();
        storage.add_prompt(greeting_prompt);

        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());

        let rendered = main_template
            .render(&args, &storage)
            .expect("Failed to render template prompt with reference");
        assert_eq!("Hello! Nice to meet you Alice!", rendered);
    }

    #[test]
    fn test_render_template_with_missing_prompt_reference() {
        let metadata = PromptMetadata::new("template".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "Message: {{prompt:missing}}".to_string());
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());

        let storage = MockStorage::new();
        let result = template.render(&args, &storage);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_template_with_nested_template_success() {
        // Create a template prompt that will be referenced
        let nested_metadata = PromptMetadata::new("nested_template".to_string(), None, vec![]);
        let nested_template_prompt = Prompt::new(
            nested_metadata,
            "This is a nested template with {{variable}}".to_string(),
        );
        let nested_template =
            PromptTemplate::new(nested_template_prompt).expect("Failed to create nested template");

        // Create a main template that references the nested template
        let main_metadata = PromptMetadata::new("main".to_string(), None, vec![]);
        let main_prompt = Prompt::new(
            main_metadata,
            "Referencing: {{prompt:nested_template}}".to_string(),
        );
        let main_template =
            PromptTemplate::new(main_prompt).expect("Failed to create main template");

        // Set up storage with the nested template prompt
        let mut storage = MockStorage::new();
        storage.add_prompt(nested_template.prompt); // Store the original prompt

        let mut args = HashMap::new();
        args.insert("variable".to_string(), "value".to_string());

        // Attempt to render, which should succeed with our new implementation
        let result = main_template.render(&args, &storage);
        assert!(result.is_ok());
        assert_eq!(
            "Referencing: This is a nested template with value",
            result.unwrap()
        );
    }

    #[test]
    fn test_render_template_with_circular_reference() {
        // Create prompts that reference each other
        let prompt_a_metadata = PromptMetadata::new("prompt_a".to_string(), None, vec![]);
        let prompt_a = Prompt::new(prompt_a_metadata, "A {{prompt:prompt_b}}".to_string());
        let template_a = PromptTemplate::new(prompt_a.clone()).expect("Failed to create template");

        let prompt_b_metadata = PromptMetadata::new("prompt_b".to_string(), None, vec![]);
        let prompt_b = Prompt::new(prompt_b_metadata, "B {{prompt:prompt_a}}".to_string());

        // Set up storage with both prompts
        let mut storage = MockStorage::new();
        storage.add_prompt(prompt_a);
        storage.add_prompt(prompt_b);

        let args = HashMap::new();

        // Try to render prompt_a, which should fail due to circular reference
        let result = template_a.render(&args, &storage);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("Circular reference detected")
        );
    }

    #[test]
    fn test_render_template_with_max_depth_exceeded() {
        // Create prompts with nesting that exceeds the maximum depth
        let prompt_level_0_metadata =
            PromptMetadata::new("prompt_level_0".to_string(), None, vec![]);
        let prompt_level_0 = Prompt::new(
            prompt_level_0_metadata,
            "Level 0 {{prompt:prompt_level_1}}".to_string(),
        );
        let template_level_0 =
            PromptTemplate::new(prompt_level_0.clone()).expect("Failed to create template");

        let prompt_level_1_metadata =
            PromptMetadata::new("prompt_level_1".to_string(), None, vec![]);
        let prompt_level_1 = Prompt::new(
            prompt_level_1_metadata,
            "Level 1 {{prompt:prompt_level_2}}".to_string(),
        );

        let prompt_level_2_metadata =
            PromptMetadata::new("prompt_level_2".to_string(), None, vec![]);
        let prompt_level_2 = Prompt::new(
            prompt_level_2_metadata,
            "Level 2 {{prompt:prompt_level_3}}".to_string(),
        );

        let prompt_level_3_metadata =
            PromptMetadata::new("prompt_level_3".to_string(), None, vec![]);
        let prompt_level_3 = Prompt::new(
            prompt_level_3_metadata,
            "Level 3 {{prompt:prompt_level_4}}".to_string(),
        );

        let prompt_level_4_metadata =
            PromptMetadata::new("prompt_level_4".to_string(), None, vec![]);
        let prompt_level_4 = Prompt::new(prompt_level_4_metadata, "Level 4".to_string());

        // Set up storage with all prompts
        let mut storage = MockStorage::new();
        storage.add_prompt(prompt_level_0);
        storage.add_prompt(prompt_level_1);
        storage.add_prompt(prompt_level_2);
        storage.add_prompt(prompt_level_3);
        storage.add_prompt(prompt_level_4);

        let args = HashMap::new();

        // Try to render prompt_level_0, which should fail due to exceeding max depth
        let result = template_level_0.render(&args, &storage);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("Maximum nesting depth of 3 exceeded")
        );
    }

    #[test]
    fn test_render_template_with_valid_depth() {
        // Create prompts with nesting that is within the maximum depth
        let prompt_level_0_metadata =
            PromptMetadata::new("prompt_level_0".to_string(), None, vec![]);
        let prompt_level_0 = Prompt::new(
            prompt_level_0_metadata,
            "Level 0 {{prompt:prompt_level_1}}".to_string(),
        );
        let template_level_0 =
            PromptTemplate::new(prompt_level_0.clone()).expect("Failed to create template");

        let prompt_level_1_metadata =
            PromptMetadata::new("prompt_level_1".to_string(), None, vec![]);
        let prompt_level_1 = Prompt::new(
            prompt_level_1_metadata,
            "Level 1 {{prompt:prompt_level_2}}".to_string(),
        );

        let prompt_level_2_metadata =
            PromptMetadata::new("prompt_level_2".to_string(), None, vec![]);
        let prompt_level_2 = Prompt::new(prompt_level_2_metadata, "Level 2".to_string());

        // Set up storage with all prompts
        let mut storage = MockStorage::new();
        storage.add_prompt(prompt_level_0);
        storage.add_prompt(prompt_level_1);
        storage.add_prompt(prompt_level_2);

        let args = HashMap::new();

        // Try to render prompt_level_0, which should succeed
        let result = template_level_0.render(&args, &storage);
        assert!(result.is_ok());
        assert_eq!("Level 0 Level 1 Level 2", result.unwrap());
    }

    #[test]
    fn test_render_template_with_variable_prompt_reference() {
        // Create a prompt that will be referenced dynamically
        let dynamic_metadata = PromptMetadata::new("greeting".to_string(), None, vec![]);
        let dynamic_prompt = Prompt::new(dynamic_metadata, "Hello {{name}}!".to_string());
        let _dynamic_template =
            PromptTemplate::new(dynamic_prompt.clone()).expect("Failed to create template");

        // Create a main template that uses a variable prompt reference
        let main_metadata = PromptMetadata::new("main".to_string(), None, vec![]);
        let main_prompt = Prompt::new(
            main_metadata,
            "Message: {{prompt_var:prompt_name}}".to_string(),
        );
        let main_template = PromptTemplate::new(main_prompt)
            .expect("Failed to create template with variable reference");

        // Set up storage with the dynamic prompt
        let mut storage = MockStorage::new();
        storage.add_prompt(dynamic_prompt);

        // Provide the argument that specifies which prompt to reference
        let mut args = HashMap::new();
        args.insert("prompt_name".to_string(), "greeting".to_string());
        args.insert("name".to_string(), "Alice".to_string());

        let rendered = main_template
            .render(&args, &storage)
            .expect("Failed to render template prompt with variable reference");
        assert_eq!("Message: Hello Alice!", rendered);
    }

    #[test]
    fn test_variable_prompt_references() {
        let metadata = PromptMetadata::new("template".to_string(), None, vec![]);
        let prompt = Prompt::new(
            metadata,
            "Use {{prompt_var:first}} and {{prompt_var:second}} for dynamic content".to_string(),
        );
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let variable_refs = template.variable_prompt_references();
        assert_eq!(variable_refs.len(), 2);
        assert!(variable_refs.contains(&"first".to_string()));
        assert!(variable_refs.contains(&"second".to_string()));
    }

    #[test]
    fn test_render_template_with_missing_variable_prompt_reference() {
        let metadata = PromptMetadata::new("template".to_string(), None, vec![]);
        let prompt = Prompt::new(
            metadata,
            "Message: {{prompt_var:missing_prompt}}".to_string(),
        );
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let mut args = HashMap::new();
        args.insert("missing_prompt".to_string(), "nonexistent".to_string());

        let storage = MockStorage::new();
        let result = template.render(&args, &storage);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_template_with_variable_prompt_reference_missing_argument() {
        let metadata = PromptMetadata::new("template".to_string(), None, vec![]);
        let prompt = Prompt::new(metadata, "Message: {{prompt_var:prompt_name}}".to_string());
        let template = PromptTemplate::new(prompt).expect("Failed to create template");

        let args = HashMap::new(); // Missing the "prompt_name" argument

        let storage = MockStorage::new();
        let result = template.render(&args, &storage);
        assert!(result.is_err());
        assert_eq!("Missing argument: prompt_name", result.unwrap_err().message);
    }

    #[test]
    fn test_render_template_with_variable_prompt_reference_circular_reference() {
        // Create prompts that reference each other circularly
        let prompt_a_metadata = PromptMetadata::new("prompt_a".to_string(), None, vec![]);
        let prompt_a = Prompt::new(prompt_a_metadata, "A {{prompt_var:ref_prompt}}".to_string());
        let template_a = PromptTemplate::new(prompt_a.clone()).expect("Failed to create template");

        let prompt_b_metadata = PromptMetadata::new("prompt_b".to_string(), None, vec![]);
        let prompt_b = Prompt::new(prompt_b_metadata, "B {{name}}".to_string());

        // Set up storage with both prompts
        let mut storage = MockStorage::new();
        storage.add_prompt(prompt_a);
        storage.add_prompt(prompt_b);

        // Set up arguments where prompt_name refers back to prompt_a (circular)
        let mut args = HashMap::new();
        args.insert("ref_prompt".to_string(), "prompt_a".to_string()); // Circular reference
        args.insert("name".to_string(), "Alice".to_string());

        // Try to render prompt_a, which should fail due to circular reference
        let result = template_a.render(&args, &storage);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("Circular reference detected")
        );
    }
}
