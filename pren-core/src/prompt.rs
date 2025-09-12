use std::error::Error;
use std::collections::HashMap;
use nom::Err as NomErr;
use crate::parser::parse_template;
use crate::registry::PromptStorage;

#[derive(Debug, Clone)]
pub struct PromptBase {
    pub name: String,
    pub content: String,
    pub tags: Vec<String>,
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

#[derive(Debug, Clone)]
pub enum Prompt {
    Simple{
        base: PromptBase,
    },
    Template {
        base: PromptBase,
        template: PromptTemplate
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PromptTemplatePart {
    Literal(String),
    Argument(String),
    PromptReference(String),
}

#[derive(Debug, Clone)]
pub struct PromptTemplate {
    pub parts: Vec<PromptTemplatePart>,
}

impl Prompt {
    pub fn new_simple(name: String, content: String, tags: Vec<String>) -> Prompt {
        Prompt::Simple {
            base: PromptBase { name, content, tags,},
        }
    }

    pub fn new_template(name: String, content: String, tags: Vec<String>) -> Result<Prompt, ParseTemplateError> {
        match parse_template(&content) {
            Ok((_, template)) => Ok(Prompt::Template {
                base: PromptBase {name, content, tags},
                template,
            }),
            Err(NomErr::Error(e)) | Err(NomErr::Failure(e)) => Err(ParseTemplateError {
                message: format!("Failed to parse template: {:?}", e),
            }),
            Err(NomErr::Incomplete(_)) => Err(ParseTemplateError {
                message: "Failed to parse template: incomplete input".to_string(),
            }),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Prompt::Simple { base } => &base.name,
            Prompt::Template { base, .. } => &base.name,
        }
    }

    pub fn content(&self) -> &str {
        match self {
            Prompt::Simple { base } => &base.content,
            Prompt::Template { base, .. } => &base.content,
        }
    }

    pub fn tags(&self) -> &Vec<String> {
        match self {
            Prompt::Simple { base } => &base.tags,
            Prompt::Template { base, .. } => &base.tags,
        }
    }

    pub fn template(&self) -> Option<&PromptTemplate> {
        match self {
            Prompt::Simple { .. } => None,
            Prompt::Template { template, .. } => Some(&template),
        }
    }

    pub fn arguments(&self) -> Option<Vec<&String>> {
        match self {
            Prompt::Simple { .. } => None,
            Prompt::Template {template, .. } => Some(template.arguments()),
        }
    }

    pub fn prompt_references(&self) -> Option<Vec<&String>> {
        match self {
            Prompt::Simple { .. } => None,
            Prompt::Template { template, .. } => Some(template.prompt_references()),
        }
    }

    pub fn render(&self, arguments: &HashMap<String, String>, storage: &dyn PromptStorage) -> Result<String, RenderTemplateError> {
        match self {
            Prompt::Simple { base } => Ok(base.content.clone()),
            Prompt::Template { template, .. } => template.render(arguments, storage),
        }
    }
}

impl PromptTemplate {
    pub fn arguments(&self) -> Vec<&String> {
        self.parts.iter().filter_map(|part| {
            if let PromptTemplatePart::Argument(arg) = part {
                Some(arg)
            } else {
                None
            }
        }).collect()
    }

    pub fn prompt_references(&self) -> Vec<&String> {
        self.parts.iter().filter_map(|part| {
            if let PromptTemplatePart::PromptReference(prompt) = part {
                Some(prompt)
            } else {
                None
            }
        }).collect()
    }

    pub fn render(&self, arguments: &HashMap<String, String>, storage: &dyn PromptStorage) -> Result<String, RenderTemplateError> {
        let mut result = String::new();
        
        for part in &self.parts {
            match part {
                PromptTemplatePart::Literal(text) => result.push_str(text),
                PromptTemplatePart::Argument(name) => {
                    match arguments.get(name) {
                        Some(value) => result.push_str(value),
                        None => return Err(RenderTemplateError {
                            message: format!("Missing argument: {}", name),
                        }),
                    }
                },
                PromptTemplatePart::PromptReference(name) => {
                    match storage.get_prompt(name) {
                        Ok(Some(prompt)) => {
                            // Render the referenced prompt with the same arguments
                            match prompt.render(arguments, storage) {
                                Ok(rendered) => result.push_str(&rendered),
                                Err(e) => return Err(RenderTemplateError {
                                    message: format!("Failed to render referenced prompt '{}': {}", name, e.message),
                                }),
                            }
                        },
                        Ok(None) => return Err(RenderTemplateError {
                            message: format!("Referenced prompt not found: {}", name),
                        }),
                        Err(e) => return Err(RenderTemplateError {
                            message: format!("Error retrieving referenced prompt '{}': {}", name, e),
                        }),
                    }
                },
            }
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_simple_prompt() {
        let name = "prompt_name";
        let content = "This is the prompt content";
        let tags = vec!["tag1".to_string(), "tag2".to_string()];
        let prompt = Prompt::new_simple(name.to_string(), content.to_string(), tags.clone());

        assert_eq!(name, prompt.name());
        assert_eq!(content, prompt.content());

        assert_eq!(2, prompt.tags().len());
        assert_eq!(tags[0], prompt.tags()[0]);
        assert_eq!(tags[1], prompt.tags()[1]);
    }

    #[test]
    fn test_new_template_prompt() {
        let name = "complex_prompt";
        let content = "Hello {{name}}, welcome to {{prompt:greeting}}! {{{{literal_braces}}}}";
        let tags = vec!["tag1".to_string(), "tag2".to_string()];

        let prompt = Prompt::new_template(name.to_string(), content.to_string(), tags.clone()).expect("Failed to create template prompt");

        assert_eq!(name, prompt.name());
        assert_eq!(content, prompt.content());

        assert_eq!(2, prompt.tags().len());
        assert_eq!(tags[0], prompt.tags()[0]);
        assert_eq!(tags[1], prompt.tags()[1]);

        // Check that it's actually a template prompt
        match &prompt {
            Prompt::Template { template, .. } => {
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
                    PromptTemplatePart::PromptReference(prompt_name) => assert_eq!("greeting", prompt_name),
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
            },
            _ => panic!("Expected Template prompt"),
        }
    }

    #[test]
    fn test_arguments_simple_prompt() {
        let simple_prompt = Prompt::new_simple(
            "simple".to_string(),
            "This is a simple prompt".to_string(),
            vec![]
        );
        assert!(simple_prompt.arguments().is_none());
    }

    #[test]
    fn test_arguments_template_prompt() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Hello {{name}}, you are {{age}} years old!".to_string(),
            vec![]
        ).expect("Failed to create template prompt");

        let args = template_prompt.arguments().expect("Expected Some(Vec<&String>)");
        assert_eq!(2, args.len());
        assert_eq!("name", args[0]);
        assert_eq!("age", args[1]);
    }

    #[test]
    fn test_arguments_template_prompt_without_args() {
        let no_args_prompt = Prompt::new_template(
            "no_args".to_string(),
            "Hello, welcome to our service! {{{{literal_braces}}}}".to_string(),
            vec![]
        ).expect("Failed to create template prompt");

        let args = no_args_prompt.arguments().expect("Expected Some(Vec<&String>)");
        assert_eq!(0, args.len());
    }

    #[test]
    fn test_prompt_references_simple() {
        let simple_prompt = Prompt::new_simple(
            "simple".to_string(),
            "This is a simple prompt".to_string(),
            vec![]
        );
        assert!(simple_prompt.prompt_references().is_none());
    }



    #[test]
    fn test_prompt_references() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Greeting: {{prompt:greeting}}, Farewell: {{prompt:farewell}}".to_string(),
            vec![]
        ).expect("Failed to create template prompt");

        let refs = template_prompt.prompt_references().expect("Expected Some(Vec<&String>)");
        assert_eq!(2, refs.len());
        assert_eq!("greeting", refs[0]);
        assert_eq!("farewell", refs[1]);
    }

    #[test]
    fn test_prompt_references_no_refs() {
        let no_refs_prompt = Prompt::new_template(
            "no_refs".to_string(),
            "Hello {{name}}, how are you? {{{{literal_braces}}}}".to_string(),
            vec![]
        ).expect("Failed to create template prompt");
        
        let refs = no_refs_prompt.prompt_references().expect("Expected Some(Vec<&String>)");
        assert_eq!(0, refs.len());
    }

    #[test]
    fn test_arguments_and_prompt_references_combined() {

        let complex_prompt = Prompt::new_template(
            "complex".to_string(),
            "Dear {{name}}, {{prompt:greeting}} {{{{literal_braces}}}} Best regards, {{signature}} from {{prompt:company}}".to_string(),
            vec![]
        ).expect("Failed to create template prompt");
        
        let args = complex_prompt.arguments().expect("Expected Some(Vec<&String>)");
        assert_eq!(2, args.len());
        assert_eq!("name", args[0]);
        assert_eq!("signature", args[1]);

        let refs = complex_prompt.prompt_references().expect("Expected Some(Vec<&String>)");
        assert_eq!(2, refs.len());
        assert_eq!("greeting", refs[0]);
        assert_eq!("company", refs[1]);
    }

    #[test]
    fn test_template_method_simple_prompt() {
        let simple_prompt = Prompt::new_simple(
            "simple".to_string(),
            "This is a simple prompt".to_string(),
            vec![]
        );
        
        assert!(simple_prompt.template().is_none());
    }

    #[test]
    fn test_template_method_template_prompt() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Hello {{name}}, welcome to {{prompt:greeting}}!".to_string(),
            vec!["test".to_string()]
        ).expect("Failed to create template prompt");

        let template = template_prompt.template().expect("Expected Some(&PromptTemplate)");
        
        assert_eq!(5, template.parts.len());
        
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
            PromptTemplatePart::PromptReference(prompt_name) => assert_eq!("greeting", prompt_name),
            _ => panic!("Expected PromptReference part"),
        }
        
        match &template.parts[4] {
            PromptTemplatePart::Literal(text) => assert_eq!("!", text),
            _ => panic!("Expected Literal part"),
        }
    }

    struct MockStorage {
        prompts: std::collections::HashMap<String, Prompt>,
    }
    
    impl MockStorage {
        fn new() -> Self {
            MockStorage {
                prompts: std::collections::HashMap::new(),
            }
        }
        
        fn add_prompt(&mut self, prompt: Prompt) {
            self.prompts.insert(prompt.name().to_string(), prompt);
        }
    }
    
    impl PromptStorage for MockStorage {
        fn save_prompt(&self, _prompt: &Prompt) -> Result<(), crate::file_storage::FileStorageError> {
            Ok(())
        }
        
        fn get_prompt(&self, name: &str) -> Result<Option<Prompt>, crate::file_storage::FileStorageError> {
            Ok(self.prompts.get(name).cloned())
        }
        
        fn get_prompts(&self) -> Result<Vec<Prompt>, crate::file_storage::FileStorageError> {
            Ok(self.prompts.values().cloned().collect())
        }
        
        fn delete_prompt(&self, _name: &str) -> Result<(), crate::file_storage::FileStorageError> {
            Ok(())
        }
        
        fn get_prompts_by_tag(&self, _tags: &[String]) -> Result<Vec<Prompt>, crate::file_storage::FileStorageError> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_render_simple_prompt() {
        let simple_prompt = Prompt::new_simple(
            "simple".to_string(),
            "This is a simple prompt".to_string(),
            vec![]
        );
        
        let mut args = std::collections::HashMap::new();
        args.insert("name".to_string(), "World".to_string());
        
        let storage = MockStorage::new();
        let rendered = simple_prompt.render(&args, &storage).expect("Failed to render simple prompt");
        assert_eq!("This is a simple prompt", rendered);
    }

    #[test]
    fn test_render_template_prompt() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Hello {{name}}, welcome!".to_string(),
            vec![]
        ).expect("Failed to create template prompt");
        
        let mut args = std::collections::HashMap::new();
        args.insert("name".to_string(), "World".to_string());
        
        let storage = MockStorage::new();
        let rendered = template_prompt.render(&args, &storage).expect("Failed to render template prompt");
        assert_eq!("Hello World, welcome!", rendered);
    }

    #[test]
    fn test_render_template_prompt_missing_argument() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Hello {{name}}, welcome!".to_string(),
            vec![]
        ).expect("Failed to create template prompt");
        
        let args = std::collections::HashMap::new();
        
        let storage = MockStorage::new();
        let result = template_prompt.render(&args, &storage);
        assert!(result.is_err());
        assert_eq!("Missing argument: name", result.unwrap_err().message);
    }

    #[test]
    fn test_render_template_prompt_multiple_arguments() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Dear {{name}}, you are {{age}} years old!".to_string(),
            vec![]
        ).expect("Failed to create template prompt");
        
        let mut args = std::collections::HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        args.insert("age".to_string(), "30".to_string());
        
        let storage = MockStorage::new();
        let rendered = template_prompt.render(&args, &storage).expect("Failed to render template prompt");
        assert_eq!("Dear Alice, you are 30 years old!", rendered);
    }

    #[test]
    fn test_render_template_prompt_with_escaped_literals() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Hello {{{{{{name}}}}}}, you are {{age}} years old!".to_string(),
            vec![]
        ).expect("Failed to create template prompt");
        
        let mut args = std::collections::HashMap::new();
        args.insert("age".to_string(), "30".to_string());
        
        let storage = MockStorage::new();
        let rendered = template_prompt.render(&args, &storage).expect("Failed to render template prompt");
        assert_eq!("Hello {{name}}, you are 30 years old!", rendered);
    }

    #[test]
    fn test_render_template_with_prompt_reference() {
        let greeting_prompt = Prompt::new_simple(
            "greeting".to_string(),
            "Hello!".to_string(),
            vec![]
        );
        
        let main_prompt = Prompt::new_template(
            "main".to_string(),
            "{{prompt:greeting}} Nice to meet you {{name}}!".to_string(),
            vec![]
        ).expect("Failed to create template prompt");
        
        let mut storage = MockStorage::new();
        storage.add_prompt(greeting_prompt);
        
        let mut args = std::collections::HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        
        let rendered = main_prompt.render(&args, &storage).expect("Failed to render template prompt with reference");
        assert_eq!("Hello! Nice to meet you Alice!", rendered);
    }

    #[test]
    fn test_render_template_with_missing_prompt_reference() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Message: {{prompt:missing}}".to_string(),
            vec![]
        ).expect("Failed to create template prompt");
        
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Alice".to_string());
        
        let storage = MockStorage::new();
        let result = template_prompt.render(&args, &storage);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Referenced prompt not found"));
    }
}