pub struct PromptBase {
    pub name: String,
    pub content: String,
    pub tags: Vec<String>
}

pub enum Prompt {
    Simple(PromptBase),
    Template(PromptBase)
}

#[derive(Debug, PartialEq)]
enum PlaceholderError {
    Invalid(String),
}

pub fn new_simple(name: String, content: String, tags: Vec<String>) -> Prompt {
    Prompt::Simple(
        PromptBase{name, content, tags}
    )
}

pub fn new_template(name: String, content: String, tags: Vec<String>) -> Prompt {
    Prompt::Template(
        PromptBase{name, content, tags}
    )
}

impl Prompt {
    fn name(&self) -> &str {
        match self {
            Prompt::Simple(base) => &base.name,
            Prompt::Template(base) => &base.name,
        }
    }

    fn content(&self) -> &str {
        match self {
            Prompt::Simple(base) => &base.content,
            Prompt::Template(base) => &base.content,
        }
    }

    fn tags(&self) -> &[String] {
        match self {
            Prompt::Simple(base) => &base.tags,
            Prompt::Template(base) => &base.tags,
        }
    }

}

fn extract_placeholders(template: &str) -> Result<Vec<String>, PlaceholderError> {
    let mut placeholders = vec![];
    let bytes = template.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        // Handle {{{{ -> skip 4 bytes
        if i + 3 < bytes.len() && bytes[i..i+4] == [b'{', b'{', b'{', b'{'] {
            let mut j = i + 4;
            while j + 3 < bytes.len() {
                if bytes[j..j+4] == [b'}', b'}', b'}', b'}'] {
                    i = j + 4;  // skip to after }}}}
                    break;
                }
                j += 1;
            }
            if j + 3 >= bytes.len() {
                // didn't find }}}} — treat rest as literal
                i = bytes.len();  // jump to end
            }
            continue;
        }

        // Handle {{ -> placeholder start
        if i+1 < bytes.len() && bytes[i..i+2] == [b'{', b'{'] {
            let mut j = i + 2;
            let mut found = false;
            while j < bytes.len() {
                // Handle }} -> placeholder end
                if j+1 < bytes.len() && bytes[j..j+2] == [b'}', b'}']{
                    let placeholder = String::from_utf8_lossy(&bytes[i+2..j]).to_string();
                    if !is_placeholder_valid(&placeholder) {
                        return Err(PlaceholderError::Invalid(placeholder))
                    }
                    placeholders.push(placeholder);

                    i = j+2;
                    found = true;
                    break;
                }
                j += 1;
            }
            if found {continue}
        }
        i += 1;
    }
    Ok(placeholders)
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
    fn test_empty_template() {
        assert_eq!(extract_placeholders(""), Ok(vec![]));
    }

    #[test]
    fn test_nothing_to_extract() {
        assert_eq!(extract_placeholders("Hello world!"), Ok(vec![]))
    }

    #[test]
    fn test_extract_one_placeholder() {
        assert_eq!(extract_placeholders("Hello my name is {{name}}!"), Ok(vec!["name".to_string()]))
    }

    #[test]
    fn test_extract_one_placeholder_with_spaces() {
        assert_eq!(extract_placeholders("Hello my name is {{ my _ name }}!"), Err(PlaceholderError::Invalid(" my _ name ".to_string())))
    }

    #[test]
    fn test_extract_n_placeholders() {
        assert_eq!(extract_placeholders("You are a write with the following personality {{personality_type}}, write an essay about {{essay-topic01}}, in the following format {{prompt:format}}."), Ok(vec!["personality_type".to_string(), "essay-topic01".to_string(), "prompt:format".to_string()]))
    }

    #[test]
    fn test_escape_opening() {
        assert_eq!(extract_placeholders("Hello my name is {{{{name}}!"), Ok(vec![]))
    }

    #[test]
    fn test_escape_ending() {
        assert_eq!(extract_placeholders("Hello my name is {{name}}}}!"), Ok(vec!["name".to_string()]))
    }

    #[test]
    fn test_extract_placeholder_with_escape() {
        assert_eq!(extract_placeholders("Hello my name is {{ {{{{name}}}} }}!"), Err(PlaceholderError::Invalid(" {{{{name".to_string())))
    }

    #[test]
    fn test_placeholder_within_escape() {
        assert_eq!(extract_placeholders("Hello my name is {{{{ {{name}} }}}}!"),  Ok(vec![]))
    }

    #[test]
    fn test_double_placeholder() {
        assert_eq!(extract_placeholders("Hello my name is {{ bla bla {{name}}!"), Err(PlaceholderError::Invalid(" bla bla {{name".to_string())))
    }

    #[test]
    fn test_placeholder_and_escape() {
        assert_eq!(extract_placeholders("Hello my name is {{name}}! How are you, {{{{your_name}}}}"), Ok(vec!["name".to_string()]))
    }
}