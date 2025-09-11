use std::error::Error;
use nom::Err as NomErr;
use crate::parser::parse_template;

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
        // Test with a complex template that has both arguments and prompt references
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
        
        // For simple prompts, template() should return None
        assert!(simple_prompt.template().is_none());
    }

    #[test]
    fn test_template_method_template_prompt() {
        let template_prompt = Prompt::new_template(
            "template".to_string(),
            "Hello {{name}}, welcome to {{prompt:greeting}}!".to_string(),
            vec!["test".to_string()]
        ).expect("Failed to create template prompt");

        // For template prompts, template() should return Some(&PromptTemplate)
        let template = template_prompt.template().expect("Expected Some(&PromptTemplate)");
        
        // Verify that the template has the expected parts
        assert_eq!(5, template.parts.len());
        
        // Check first part is a literal
        match &template.parts[0] {
            PromptTemplatePart::Literal(text) => assert_eq!("Hello ", text),
            _ => panic!("Expected Literal part"),
        }
        
        // Check second part is an argument
        match &template.parts[1] {
            PromptTemplatePart::Argument(arg) => assert_eq!("name", arg),
            _ => panic!("Expected Argument part"),
        }
        
        // Check third part is a literal
        match &template.parts[2] {
            PromptTemplatePart::Literal(text) => assert_eq!(", welcome to ", text),
            _ => panic!("Expected Literal part"),
        }
        
        // Check fourth part is a prompt reference
        match &template.parts[3] {
            PromptTemplatePart::PromptReference(prompt_name) => assert_eq!("greeting", prompt_name),
            _ => panic!("Expected PromptReference part"),
        }
        
        // Check fifth part is a literal
        match &template.parts[4] {
            PromptTemplatePart::Literal(text) => assert_eq!("!", text),
            _ => panic!("Expected Literal part"),
        }
    }
}