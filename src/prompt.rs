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
    fn is_simple(&self) -> bool {
        matches!(self, Prompt::Simple { .. })
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