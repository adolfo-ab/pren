use std::collections::HashMap;
use std::io::Error;

#[derive(Debug, Clone)]
pub enum TemplatePart {
    Literal(String),
    Argument(String),
    PromptReference(String),
}

#[derive(Debug, Clone)]
pub struct ParsedTemplate {
    pub parts: Vec<TemplatePart>,
}

pub struct PromptBase {
    pub name: String,
    pub tags: Vec<String>
}

pub enum Prompt {
    Simple {
        base: PromptBase,
        content: String,
    },
    Template {
        base: PromptBase,
        template: ParsedTemplate,
    }
}

#[derive(Debug, PartialEq)]
pub enum PromptError {
    InvalidArgument(String),
    MissingValue(String),
    PromptNotFound(String),
    PromptNotSimple(String),
}

pub fn new_simple(name: String, content: String, tags: Vec<String>) -> Prompt {
    Prompt::Simple {
        base: PromptBase { name, tags },
        content,
    }
}

pub fn new_template(name: String, content: String, tags: Vec<String>) -> Result<Prompt, PromptError> {
    let template = parse_template(&content)?;
    Ok(Prompt::Template {
        base: PromptBase { name, tags },
        template,
    })
}

pub trait PromptRegistry {
    fn get_prompt(&self, name: String) -> Option<&Prompt>;
}

impl Prompt {
    pub fn name(&self) -> &str {
        match self {
            Prompt::Simple { base, .. } => &base.name,
            Prompt::Template { base, .. } => &base.name,
        }
    }

    pub fn tags(&self) -> &[String] {
        match self {
            Prompt::Simple { base, .. } => &base.tags,
            Prompt::Template { base, .. } => &base.tags,
        }
    }

    pub fn render<R: PromptRegistry>(&self, values: &HashMap<String, String>, registry: &R) -> Result<String, PromptError> {
        match self {
            Prompt::Simple { content, .. } => Ok(content.clone()),
            Prompt::Template { template, .. } => template.render(values, registry),
        }
    }

    pub fn arguments(&self) -> Vec<String> {
        match self {
            Prompt::Simple { .. } => vec![],
            Prompt::Template { template, .. } => template.arguments(),
        }
    }

    pub fn prompt_references(&self) -> Vec<String> {
        match self {
            Prompt::Simple { .. } => vec![],
            Prompt::Template { template, .. } => template.prompt_references(),
        }
    }
}

impl ParsedTemplate {
    pub fn render<R: PromptRegistry>(&self, values: &HashMap<String, String>, registry: &R) -> Result<String, PromptError> {
        let mut result = String::new();

        for part in &self.parts {
            match part {
                TemplatePart::Literal(text) => result.push_str(text),
                TemplatePart::Argument(name) => {
                    match values.get(name) {
                        Some(value) => result.push_str(value),
                        None => return Err(PromptError::MissingValue(name.clone())),
                    }
                },
                TemplatePart::PromptReference(name) => {
                    match registry.get_prompt(name.to_string()) {
                        Some(Prompt::Simple {content, ..}) => result.push_str(content),
                        Some(Prompt::Template { .. }) => return Err(PromptError::PromptNotSimple(name.clone())),
                        None => return Err(PromptError::PromptNotFound(name.clone()))
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn arguments(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                TemplatePart::Argument(name) => Some(name.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn prompt_references(&self) -> Vec<String> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                TemplatePart::PromptReference(name) => Some(name.clone()),
                _ => None,
            })
            .collect()
    }
}

fn parse_template(template: &str) -> Result<ParsedTemplate, PromptError> {
    let mut parts = Vec::new();
    let bytes = template.as_bytes();
    let mut current_literal = String::new();

    let mut i = 0;
    while i < bytes.len() {
        // Handle {{{{ -> escaped braces, treat as literal
        if i + 3 < bytes.len() && bytes[i..i+4] == [b'{', b'{', b'{', b'{'] {
            let mut j = i + 4;
            current_literal.push_str("{{");

            while j + 3 < bytes.len() {
                if bytes[j..j+4] == [b'}', b'}', b'}', b'}'] {
                    current_literal.push_str("}}");
                    i = j + 4;
                    break;
                }
                current_literal.push(bytes[j] as char);
                j += 1;
            }
            if j + 3 >= bytes.len() {
                // didn't find }}}} — treat rest as literal
                current_literal.push_str(&String::from_utf8_lossy(&bytes[j..]));
                i = bytes.len();
            }
            continue;
        }

        // Handle {{ -> placeholder start
        if i + 1 < bytes.len() && bytes[i..i+2] == [b'{', b'{'] {
            let mut j = i + 2;
            let mut found = false;

            while j < bytes.len() {
                if j + 1 < bytes.len() && bytes[j..j+2] == [b'}', b'}'] {
                    let placeholder_name = String::from_utf8_lossy(&bytes[i+2..j]).to_string();
                    if !is_placeholder_valid(&placeholder_name) {
                        return Err(PromptError::InvalidArgument(placeholder_name));
                    }

                    // Push any accumulated literal text
                    if !current_literal.is_empty() {
                        parts.push(TemplatePart::Literal(current_literal.clone()));
                        current_literal.clear();
                    }

                    // Push the placeholder
                    if placeholder_name.starts_with("prompt:") {
                        let prompt_name = placeholder_name.strip_prefix("prompt:").unwrap();
                        parts.push(TemplatePart::PromptReference(prompt_name.to_string()));
                    } else {
                        parts.push(TemplatePart::Argument(placeholder_name));
                    }

                    i = j + 2;
                    found = true;
                    break;
                }
                j += 1;
            }

            if !found {
                // No closing }}, treat as literal
                current_literal.push_str("{{");
                i += 2;
            }
            continue;
        }

        // Regular character
        current_literal.push(bytes[i] as char);
        i += 1;
    }

    // Push any remaining literal text
    if !current_literal.is_empty() {
        parts.push(TemplatePart::Literal(current_literal));
    }

    Ok(ParsedTemplate { parts })
}

fn is_placeholder_valid(name: &str) -> bool {
    if name.is_empty() || name.len() > 64  {
        return false;
    }

    for b in name.bytes() {
        if !((b >= b'A' && b <= b'Z') ||
            (b >= b'a' && b <= b'z') ||
            (b >= b'0' && b <= b'9') ||
            b == b'_' ||
            b == b'-' ||
            b == b':') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_prompt_render() {
        let prompt = new_simple("test".to_string(), "Hello world!".to_string(), vec![]);
        let values = HashMap::new();
        assert_eq!(prompt.render(&values).unwrap(), "Hello world!");
    }

    #[test]
    fn test_template_creation() {
        let prompt = new_template(
            "test".to_string(),
            "Hello {{name}}, I am {{my_name}}, this is a prompt: {{prompt:my_prompt}}!".to_string(),
            vec![]
        ).unwrap();

        assert_eq!(prompt.arguments(), vec!["name", "my_name"]);
        assert_eq!(prompt.prompt_references(), vec!["my_prompt"]);
    }

    #[test]
    fn test_template_render() {
        let prompt = new_template(
            "test".to_string(),
            "Hello {{name}}!".to_string(),
            vec![]
        ).unwrap();

        let mut values = HashMap::new();
        values.insert("name".to_string(), "Alice".to_string());

        assert_eq!(prompt.render(&values).unwrap(), "Hello Alice!");
    }

    #[test]
    fn test_template_missing_value() {
        let prompt = new_template(
            "test".to_string(),
            "Hello {{name}}!".to_string(),
            vec![]
        ).unwrap();

        let values = HashMap::new();

        match prompt.render(&values) {
            Err(PromptError::MissingValue(name)) => assert_eq!(name, "name"),
            _ => panic!("Expected MissingValue error"),
        }
    }

    #[test]
    fn test_complex_template() {
        let prompt = new_template(
            "test".to_string(),
            "You are a {{role}} with {{trait}}. Write about {{topic}}.".to_string(),
            vec![]
        ).unwrap();

        let mut values = HashMap::new();
        values.insert("role".to_string(), "writer".to_string());
        values.insert("trait".to_string(), "creativity".to_string());
        values.insert("topic".to_string(), "AI".to_string());

        assert_eq!(
            prompt.render(&values).unwrap(),
            "You are a writer with creativity. Write about AI."
        );
    }

    #[test]
    fn test_escaped_braces() {
        let prompt = new_template(
            "test".to_string(),
            "Use {{{{code}}}} tags around {{content}}.".to_string(),
            vec![]
        ).unwrap();

        let mut values = HashMap::new();
        values.insert("content".to_string(), "hello".to_string());

        assert_eq!(
            prompt.render(&values).unwrap(),
            "Use {{code}} tags around hello."
        );
    }

    #[test]
    fn test_unfinished_escape() {
        let prompt = new_template(
            "test".to_string(),
            "Use {{{{code tags around content.".to_string(),
            vec![]
        ).unwrap();

        let values = HashMap::new();

        assert_eq!(
            prompt.render(&values).unwrap(),
            "Use {{code tags around content."
        );
    }

    #[test]
    fn test_unfinished_placeholder() {
        let prompt = new_template(
            "test".to_string(),
            "Use {{code tags around content.".to_string(),
            vec![]
        ).unwrap();

        let values = HashMap::new();

        assert_eq!(
            prompt.render(&values).unwrap(),
            "Use {{code tags around content."
        );
    }

    #[test]
    fn test_invalid_placeholder() {
        let result = new_template(
            "test".to_string(),
            "Hello {{ invalid name }}!".to_string(),
            vec![]
        );

        match result {
            Err(PromptError::InvalidArgument(name)) => assert_eq!(name, " invalid name "),
            _ => panic!("Expected InvalidPlaceholder error"),
        }
    }
}