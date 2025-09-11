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
}