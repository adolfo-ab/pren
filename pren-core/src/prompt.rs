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
//! use pren_core::prompt::Prompt;
//!
//! let prompt = Prompt::new(
//!     "greeting".to_string(),
//!     "Hello, world!".to_string(),
//!     vec!["example".to_string()]
//! ).expect("Failed to create prompt");
//! ```
//!
//! Creating a template prompt:
//!
//! ```rust
//! use pren_core::prompt::Prompt;
//!
//! let prompt = Prompt::new(
//!     "personal_greeting".to_string(),
//!     "Hello {{name}}, welcome to {{prompt:service_name}}!".to_string(),
//!     vec!["example".to_string()]
//! ).expect("Failed to create template prompt");
//! ```

use crate::parser::parse_template;
use crate::storage::PromptStorage;
use nom::Err as NomErr;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use chrono::{DateTime, Local};

/// Maximum allowed nesting depth for prompt templates
const MAX_NESTING_DEPTH: usize = 3;


#[derive(Debug, Clone)]
pub struct Prompt {
    pub name: String,
    pub content: String,
    pub template: PromptTemplate,
    pub tags: Vec<String>,
    pub creation_date: DateTime<Local>
}

#[derive(Debug)]
pub struct ParseTemplateError {
    pub message: String,
}

impl std::fmt::Display for ParseTemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Parse template error: {}", self.message)
    }
}

impl Error for ParseTemplateError {}

#[derive(Debug)]
pub struct RenderTemplateError {
    pub message: String,
}

impl std::fmt::Display for RenderTemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Render template error: {}", self.message)
    }
}

impl Error for RenderTemplateError {}

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
#[derive(Debug, Clone, PartialEq)]
pub enum PromptTemplatePart {
    /// Literal text that is rendered as-is.
    Literal(String),
    /// An argument placeholder that gets replaced with a value at render time.
    Argument(String),
    /// A reference to another prompt that gets rendered at render time.
    PromptReference(String),
}

/// A parsed template with parts that can be literals, arguments, or prompt references.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// The parts that make up the template.
    pub parts: Vec<PromptTemplatePart>,
}

impl Prompt {
    /// Creates a new template prompt.
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
    pub fn new(
        name: String,
        content: String,
        tags: Vec<String>,
    ) -> Result<Prompt, ParseTemplateError> {
        match parse_template(&content) {
            Ok((_, template)) => Ok(Prompt {
                name,
                content,
                template,
                tags,
                creation_date: Local::now(),
            }),
            Err(NomErr::Error(e)) | Err(NomErr::Failure(e)) => Err(ParseTemplateError {
                message: format!("Failed to parse template: {:?}", e),
            }),
            Err(NomErr::Incomplete(_)) => Err(ParseTemplateError {
                message: "Failed to parse template: incomplete input".to_string(),
            }),
        }
    }

    pub fn is_simple(content: String) -> Result<bool, ParseTemplateError> {
        match parse_template(&content) {
            Ok((_, template)) => {
                Ok(template.arguments().len() == 0 && template.prompt_references().len() == 0)
            }
            Err(NomErr::Error(e)) | Err(NomErr::Failure(e)) => Err(ParseTemplateError {
                message: format!("Failed to parse template: {:?}", e),
            }),
            Err(NomErr::Incomplete(_)) => Err(ParseTemplateError {
                message: "Failed to parse template: incomplete input".to_string(),
            }),
        }
    }

    /// Renders a prompt with the given arguments.
    ///
    /// For simple prompts, this returns the content as-is.
    /// For template prompts, this substitutes placeholders with the provided values
    /// and resolves prompt references using the provided storage.
    ///
    /// # Arguments
    ///
    /// * `arguments` - A map of argument names to values for template substitution.
    /// * `storage` - A storage implementation for resolving prompt references.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The rendered prompt content.
    /// * `Err(RenderTemplateError)` - If there was an error during rendering.
    pub fn render<S: PromptStorage>(
        &self,
        arguments: &HashMap<String, String>,
        storage: &S,
    ) -> Result<String, RenderTemplateError> {
        self.template.render(arguments, storage)
    }
}

impl PromptTemplate {
    pub fn arguments(&self) -> Vec<&String> {
        self.parts
            .iter()
            .filter_map(|part| {
                if let PromptTemplatePart::Argument(arg) = part {
                    Some(arg)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn prompt_references(&self) -> Vec<&String> {
        self.parts
            .iter()
            .filter_map(|part| {
                if let PromptTemplatePart::PromptReference(prompt) = part {
                    Some(prompt)
                } else {
                    None
                }
            })
            .collect()
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
                    // Validate before resolving the prompt reference
                    context.enter_prompt(name)?;

                    match storage.get_prompt(name) {
                        Ok(prompt) => {
                            match prompt.template.render_internal(arguments, storage, context) {
                                Ok(rendered) => result.push_str(&rendered),
                                Err(e) => {
                                    context.exit_prompt(name);
                                    return Err(RenderTemplateError {
                                        message: format!(
                                            "Failed to render referenced prompt '{}': {}",
                                            name, e.message
                                        ),
                                    });
                                }
                            }
                        }
                        Err(e) => {
                            context.exit_prompt(name);
                            return Err(RenderTemplateError {
                                message: format!(
                                    "Error retrieving referenced prompt '{}': {}",
                                    name, e
                                ),
                            });
                        }
                    }

                    // Exit the prompt after successful rendering
                    context.exit_prompt(name);
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Error;

    #[test]
    fn test_new_simple_prompt() {
        let name = "prompt_name";
        let content = "This is the prompt content";
        let tags = vec!["tag1".to_string(), "tag2".to_string()];
        let result = Prompt::new(name.to_string(), content.to_string(), tags.clone());

        assert!(result.is_ok());

        let prompt = result.unwrap();

        assert_eq!(name, prompt.name);
        assert_eq!(content, prompt.content);
        assert_eq!(1, prompt.template.parts.len());
        assert_eq!(2, prompt.tags.len());
        assert_eq!(tags[0], prompt.tags[0]);
        assert_eq!(tags[1], prompt.tags[1]);
    }

    #[test]
    fn test_new_template_prompt() {
        let name = "complex_prompt";
        let content = "Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal_braces}}}}";
        let tags = vec!["tag1".to_string(), "tag2".to_string()];

        let prompt = Prompt::new(name.to_string(), content.to_string(), tags.clone())
            .expect("Failed to create template prompt");

        assert_eq!(name, prompt.name);
        assert_eq!(content, prompt.content);

        assert_eq!(2, prompt.tags.len());
        assert_eq!(tags[0], prompt.tags[0]);
        assert_eq!(tags[1], prompt.tags[1]);

        // Check that it's actually a template prompt
        assert_eq!(6, prompt.template.parts.len());

        // Check each part
        match &prompt.template.parts[0] {
            PromptTemplatePart::Literal(text) => assert_eq!("Hello ", text),
            _ => panic!("Expected Literal part"),
        }

        match &prompt.template.parts[1] {
            PromptTemplatePart::Argument(arg) => assert_eq!("name", arg),
            _ => panic!("Expected Argument part"),
        }

        match &prompt.template.parts[2] {
            PromptTemplatePart::Literal(text) => assert_eq!(", welcome to ", text),
            _ => panic!("Expected Literal part"),
        }

        match &prompt.template.parts[3] {
            PromptTemplatePart::PromptReference(prompt_name) => {
                assert_eq!("greeting", prompt_name)
            }
            _ => panic!("Expected PromptReference part"),
        }

        match &prompt.template.parts[4] {
            PromptTemplatePart::Literal(text) => assert_eq!("! ", text),
            _ => panic!("Expected Literal part"),
        }

        match &prompt.template.parts[5] {
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
            self.prompts.insert(prompt.name.clone(), prompt);
        }
    }

    impl PromptStorage for MockStorage {
        type Error = Error;

        fn save_prompt(&self, _prompt: &Prompt) -> Result<(), Error> {
            Ok(())
        }

        fn get_prompt(&self, name: &str) -> Result<Prompt, Error> {
            match self.prompts.get(name) {
                Some(prompt) => Ok(prompt.clone()),
                None => Err(Error),
            }
        }

        fn get_prompts(&self) -> Result<Vec<Prompt>, Error> {
            Ok(self.prompts.values().cloned().collect())
        }

        fn delete_prompt(&self, _name: &str) -> Result<(), Error> {
            Ok(())
        }

        fn get_prompts_by_tag(&self, _tags: &[String]) -> Result<Vec<Prompt>, Error> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_render_simple_prompt() {
        let simple_prompt = Prompt::new(
            "simple".to_string(),
            "This is a simple prompt".to_string(),
            vec![],
        )
        .expect("Failed to create simple prompt");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "World".to_string());

        let storage = MockStorage::new();
        let rendered = simple_prompt
            .render(&args, &storage)
            .expect("Failed to render simple prompt");
        assert_eq!("This is a simple prompt", rendered);
    }

    #[test]
    fn test_render_template_prompt() {
        let template_prompt = Prompt::new(
            "template".to_string(),
            "Hello {{name}}, welcome!".to_string(),
            vec![],
        )
        .expect("Failed to create template prompt");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "World".to_string());

        let storage = MockStorage::new();
        let rendered = template_prompt
            .render(&args, &storage)
            .expect("Failed to render template prompt");
        assert_eq!("Hello World, welcome!", rendered);
    }

    #[test]
    fn test_render_template_prompt_missing_argument() {
        let template_prompt = Prompt::new(
            "template".to_string(),
            "Hello {{name}}, welcome!".to_string(),
            vec![],
        )
        .expect("Failed to create template prompt");

        let args = HashMap::new();

        let storage = MockStorage::new();
        let result = template_prompt.render(&args, &storage);
        assert!(result.is_err());
        assert_eq!("Missing argument: name", result.unwrap_err().message);
    }

    #[test]
    fn test_render_template_prompt_multiple_arguments() {
        let template_prompt = Prompt::new(
            "template".to_string(),
            "Dear {{name}}, you are {{age}} years old!".to_string(),
            vec![],
        )
        .expect("Failed to create template prompt");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        args.insert("age".to_string(), "30".to_string());

        let storage = MockStorage::new();
        let rendered = template_prompt
            .render(&args, &storage)
            .expect("Failed to render template prompt");
        assert_eq!("Dear Alice, you are 30 years old!", rendered);
    }

    #[test]
    fn test_render_template_prompt_with_escaped_literals() {
        let template_prompt = Prompt::new(
            "template".to_string(),
            "Hello {{{{{{name}}}}}}, you are {{age}} years old!".to_string(),
            vec![],
        )
        .expect("Failed to create template prompt");

        let mut args = HashMap::new();
        args.insert("age".to_string(), "30".to_string());

        let storage = MockStorage::new();
        let rendered = template_prompt
            .render(&args, &storage)
            .expect("Failed to render template prompt");
        assert_eq!("Hello {{name}}, you are 30 years old!", rendered);
    }

    #[test]
    fn test_render_template_with_prompt_reference() {
        let greeting_prompt = Prompt::new("greeting".to_string(), "Hello!".to_string(), vec![])
            .expect("Failed to create greeting prompt");

        let main_prompt = Prompt::new(
            "main".to_string(),
            "{{prompt:greeting}} Nice to meet you {{name}}!".to_string(),
            vec![],
        )
        .expect("Failed to create template prompt");

        let mut storage = MockStorage::new();
        storage.add_prompt(greeting_prompt);

        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());

        let rendered = main_prompt
            .render(&args, &storage)
            .expect("Failed to render template prompt with reference");
        assert_eq!("Hello! Nice to meet you Alice!", rendered);
    }

    #[test]
    fn test_render_template_with_missing_prompt_reference() {
        let template_prompt = Prompt::new(
            "template".to_string(),
            "Message: {{prompt:missing}}".to_string(),
            vec![],
        )
        .expect("Failed to create template prompt");

        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());

        let storage = MockStorage::new();
        let result = template_prompt.render(&args, &storage);
        assert!(result.is_err());
    }

    #[test]
    fn test_render_template_with_nested_template_success() {
        // Create a template prompt that will be referenced
        let nested_template_prompt = Prompt::new(
            "nested_template".to_string(),
            "This is a nested template with {{variable}}".to_string(),
            vec![],
        )
        .expect("Failed to create nested template prompt");

        // Create a main template that references the nested template
        let main_prompt = Prompt::new(
            "main".to_string(),
            "Referencing: {{prompt:nested_template}}".to_string(),
            vec![],
        )
        .expect("Failed to create main template prompt");

        // Set up storage with the nested template prompt
        let mut storage = MockStorage::new();
        storage.add_prompt(nested_template_prompt);

        let mut args = HashMap::new();
        args.insert("variable".to_string(), "value".to_string());

        // Attempt to render, which should succeed with our new implementation
        let result = main_prompt.render(&args, &storage);
        assert!(result.is_ok());
        assert_eq!(
            "Referencing: This is a nested template with value",
            result.unwrap()
        );
    }

    #[test]
    fn test_render_template_with_circular_reference() {
        // Create prompts that reference each other
        let prompt_a = Prompt::new(
            "prompt_a".to_string(),
            "A {{prompt:prompt_b}}".to_string(),
            vec![],
        )
        .expect("Failed to create prompt_a");

        let prompt_b = Prompt::new(
            "prompt_b".to_string(),
            "B {{prompt:prompt_a}}".to_string(),
            vec![],
        )
        .expect("Failed to create prompt_b");

        // Set up storage with both prompts
        let mut storage = MockStorage::new();
        storage.add_prompt(prompt_a);
        storage.add_prompt(prompt_b);

        let args = HashMap::new();

        // Try to render prompt_a, which should fail due to circular reference
        let result = storage
            .get_prompt("prompt_a")
            .unwrap()
            .render(&args, &storage);
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
        let prompt_level_0 = Prompt::new(
            "prompt_level_0".to_string(),
            "Level 0 {{prompt:prompt_level_1}}".to_string(),
            vec![],
        )
        .expect("Failed to create prompt_level_0");

        let prompt_level_1 = Prompt::new(
            "prompt_level_1".to_string(),
            "Level 1 {{prompt:prompt_level_2}}".to_string(),
            vec![],
        )
        .expect("Failed to create prompt_level_1");

        let prompt_level_2 = Prompt::new(
            "prompt_level_2".to_string(),
            "Level 2 {{prompt:prompt_level_3}}".to_string(),
            vec![],
        )
        .expect("Failed to create prompt_level_2");

        let prompt_level_3 = Prompt::new(
            "prompt_level_3".to_string(),
            "Level 3 {{prompt:prompt_level_4}}".to_string(),
            vec![],
        )
        .expect("Failed to create prompt_level_3");

        let prompt_level_4 =
            Prompt::new("prompt_level_4".to_string(), "Level 4".to_string(), vec![])
                .expect("Failed to create prompt_level_4");

        // Set up storage with all prompts
        let mut storage = MockStorage::new();
        storage.add_prompt(prompt_level_0);
        storage.add_prompt(prompt_level_1);
        storage.add_prompt(prompt_level_2);
        storage.add_prompt(prompt_level_3);
        storage.add_prompt(prompt_level_4);

        let args = HashMap::new();

        // Try to render prompt_level_0, which should fail due to exceeding max depth
        let result = storage
            .get_prompt("prompt_level_0")
            .unwrap()
            .render(&args, &storage);
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
        let prompt_level_0 = Prompt::new(
            "prompt_level_0".to_string(),
            "Level 0 {{prompt:prompt_level_1}}".to_string(),
            vec![],
        )
        .expect("Failed to create prompt_level_0");

        let prompt_level_1 = Prompt::new(
            "prompt_level_1".to_string(),
            "Level 1 {{prompt:prompt_level_2}}".to_string(),
            vec![],
        )
        .expect("Failed to create prompt_level_1");

        let prompt_level_2 =
            Prompt::new("prompt_level_2".to_string(), "Level 2".to_string(), vec![])
                .expect("Failed to create prompt_level_2");

        // Set up storage with all prompts
        let mut storage = MockStorage::new();
        storage.add_prompt(prompt_level_0);
        storage.add_prompt(prompt_level_1);
        storage.add_prompt(prompt_level_2);

        let args = HashMap::new();

        // Try to render prompt_level_0, which should succeed
        let result = storage
            .get_prompt("prompt_level_0")
            .unwrap()
            .render(&args, &storage);
        assert!(result.is_ok());
        assert_eq!("Level 0 Level 1 Level 2", result.unwrap());
    }
}
