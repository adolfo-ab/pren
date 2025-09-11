use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while_m_n};
use nom::combinator::{all_consuming, map, rest, verify};
use nom::IResult;
use nom::multi::{many0};
use nom::Parser;
use nom::sequence::delimited;
use crate::prompt::{PromptTemplate, PromptTemplatePart};

pub fn parse_template(input: &str) -> IResult<&str, PromptTemplate> {
    all_consuming(map(many0(parse_element), |parts| PromptTemplate { parts })).parse(input)
}

pub fn parse_element(input: &str) -> IResult<&str, PromptTemplatePart> {
    alt((
        map(parse_escaped_literal, |text| PromptTemplatePart::Literal(text.to_string())),
        map(parse_prompt_reference, |name| PromptTemplatePart::PromptReference(name.to_string())),
        map(parse_argument, |name| PromptTemplatePart::Argument(name.to_string())),
        map(parse_literal_text, |text| PromptTemplatePart::Literal(text.to_string())),
    )).parse(input)
}

pub fn parse_literal_text(input: &str) -> IResult<&str, &str> {
    verify(
        alt((
            take_until("{{"),
            rest,
        )),
        |s: &&str| !s.is_empty(),
    ).parse(input)
}

pub fn parse_argument(input: &str) -> IResult<&str, &str> {
    delimited(tag("{{"), identifier, tag("}}")).parse(input)
}

pub fn parse_prompt_reference(input: &str) -> IResult<&str, &str> {
    delimited(tag("{{prompt:"), identifier, tag("}}")).parse(input)
}

pub fn parse_escaped_literal(input: &str) -> IResult<&str, &str> {
    delimited(tag("{{{{"), take_until("}}}}"), tag("}}}}")).parse(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    // Limit identifiers to 1-64 characters with alphanumeric, dash, underscore
    take_while_m_n(
        1,
        64,
        |c: char| c.is_alphanumeric() || c == '-' || c == '_'
    ).parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse_literal_text("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_literal_text() {
        let result = parse_literal_text("Hello!");
        assert_eq!(result, Ok(("", "Hello!")));
    }

    #[test]
    fn test_parse_argument() {
        let result = parse_argument("{{topic}} is the subject");
        assert_eq!(result, Ok((" is the subject", "topic")));
    }

    #[test]
    fn test_parse_consecutive_variables() {
        let result = parse_template("{{a}}{{b}}{{prompt:c}}");
        assert!(result.is_ok());
        let (remaining, template) = result.unwrap();
        assert_eq!(remaining, "");
        assert_eq!(template.parts.len(), 3);
    }

    #[test]
    fn test_parse_variables_at_boundaries() {
        let result = parse_template("{{start}}middle{{end}}");
        assert!(result.is_ok());
        let (remaining, template) = result.unwrap();
        assert_eq!(remaining, "");
        assert_eq!(template.parts.len(), 3);
    }

    #[test]
    fn test_parse_incomplete_templates() {
        let result = parse_template("Hello {{name"); // Missing closing }}
        assert!(result.is_err());

        let result = parse_template("{{prompt:test"); // Missing closing }}
        assert!(result.is_err());

        let result = parse_template("{{{{hello"); // Missing closing }}}}
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_with_whitespace() {
        // Currently your parser doesn't allow whitespace in identifiers
        let result = parse_argument("{{ name }}");
        assert!(result.is_err(), "Whitespace should not be allowed");

        let result = parse_prompt_reference("{{prompt: test }}");
        assert!(result.is_err(), "Whitespace should not be allowed");
    }

    #[test]
    fn test_parse_special_characters_in_literals() {
        let result = parse_template("Hello {name} with braces but not template syntax");
        assert!(result.is_ok());
        // Should parse as literal text, not as a template element
    }

    #[test]
    fn test_parse_invalid_argument() {
        let result = parse_argument("{{to/pic}} is the subject");
        assert!(result.is_err(), "Expected parse to fail due to non-alphanumeric character");
    }

    #[test]
    fn test_parse_empty_identifier() {
        let result = parse_argument("{{}}");
        assert!(result.is_err(), "Empty identifier should fail");

        let result = parse_prompt_reference("{{prompt:}}");
        assert!(result.is_err(), "Empty prompt reference should fail");
    }

    #[test]
    fn test_parse_only_escaped_literals() {
        let result = parse_template("{{{{he{ll}o}}}}");
        assert!(result.is_ok());
        let (remaining, template) = result.unwrap();
        assert_eq!(remaining, "");
        assert_eq!(template.parts.len(), 1);
        assert!(matches!(template.parts[0], PromptTemplatePart::Literal(_)));
    }

    #[test]
    fn test_parse_prompt_reference() {
        let result = parse_prompt_reference("{{prompt:basic_prompt}} is the prompt");
        assert_eq!(result, Ok((" is the prompt", "basic_prompt")));
    }

    #[test]
    fn test_parse_invalid_prompt_reference() {
        let result = parse_prompt_reference("{{prompt:basic:prompt}} is the prompt");
        assert!(result.is_err(), "Expected parse to fail due to non-alphanumeric character");
    }

    #[test]
    fn test_parse_escaped_literal() {
        let result = parse_escaped_literal("{{{{he{llo wo}rld}}}} more text");
        assert_eq!(result, Ok((" more text", "he{llo wo}rld")));
    }

    #[test]
    fn test_parse_element_argument() {
        let result = parse_element("{{username}}");
        assert_eq!(result, Ok(("", PromptTemplatePart::Argument(String::from("username")))));
    }

    #[test]
    fn test_parse_element_invalid_argument() {
        let result = parse_element("{{user&name}}");
        assert!(result.is_err(), "Expected parse to fail due to non-alphanumeric character");
    }

    #[test]
    fn test_parse_element_prompt_reference() {
        let result = parse_element("{{prompt:username}}");
        assert_eq!(result, Ok(("", PromptTemplatePart::PromptReference(String::from("username")))));
    }

    #[test]
    fn test_parse_element_invalid_prompt_reference() {
        let result = parse_element("{{prompt:u$ername}}");
        assert!(result.is_err(), "Expected parse to fail due to non-alphanumeric character");
    }

    #[test]
    fn test_parse_element_literal() {
        let result = parse_element("username");
        assert_eq!(result, Ok(("", PromptTemplatePart::Literal(String::from("username")))));
    }

    #[test]
    fn test_parse_element_escaped_literal() {
        let result = parse_element("{{{{hello{{username}}bye}}}}");
        assert_eq!(result, Ok(("", PromptTemplatePart::Literal(String::from("hello{{username}}bye")))));
    }

    #[test]
    fn test_parse_template() {
        let result = parse_template("Hello {{name}}, welcome to {{prompt:greeting}}!");
        assert!(result.is_ok());
        let (remaining, template) = result.unwrap();
        assert_eq!(remaining, "");
        assert_eq!(template.parts.len(), 5);
    }

    #[test]
    fn test_parse_invalid_template() {
        let result = parse_template("Hello {{n@me}}, welcome to {{prompt:greeting}}!");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_template_with_escaped_literals() {
        let result = parse_template("Hello {{{{name}}}} is not a variable, but {{real_name}} is");
        assert!(result.is_ok());
        let (remaining, template) = result.unwrap();
        assert_eq!(remaining, "");
        assert_eq!(template.parts.len(), 5); // Literal, Literal, Argument
    }

    #[test]
    fn test_parse_identifier_max_length() {
        // Test maximum allowed length (64 chars)
        let max_length_id = "a".repeat(64);
        let input = format!("{{{{{}}}}}", max_length_id); // Changed to double braces
        let result = parse_argument(&input);
        assert!(result.is_ok(), "64-character identifier should work");
        assert_eq!(result.unwrap().1, max_length_id.as_str());
    }

    #[test]
    fn test_parse_identifier_too_long() {
        // Test beyond maximum length (65 chars)
        let too_long_id = "a".repeat(65);
        let input = format!("{{{{{}}}}}", too_long_id); // Changed to double braces
        let result = parse_argument(&input);
        assert!(result.is_err(), "65-character identifier should fail");
    }

    #[test]
    fn test_parse_prompt_reference_max_length() {
        // Test maximum allowed length for prompt references
        let max_length_id = "a".repeat(64);
        let input = format!("{{{{prompt:{}}}}}", max_length_id); // Changed to double braces
        let result = parse_prompt_reference(&input);
        assert!(result.is_ok(), "64-character prompt reference should work");
        assert_eq!(result.unwrap().1, max_length_id.as_str());
    }

    #[test]
    fn test_parse_prompt_reference_too_long() {
        // Test beyond maximum length for prompt references
        let too_long_id = "a".repeat(65);
        let input = format!("{{{{prompt:{}}}}}", too_long_id); // Changed to double braces
        let result = parse_prompt_reference(&input);
        assert!(result.is_err(), "65-character prompt reference should fail");
    }

    #[test]
    fn test_parse_minimum_length() {
        // Test minimum length (1 char)
        let result = parse_argument("{{a}}"); // Already correct
        assert!(result.is_ok(), "1-character identifier should work");
        assert_eq!(result.unwrap().1, "a");
    }

    #[test]
    fn test_parse_edge_case_lengths() {
        // Test various edge case lengths
        for length in [1, 2, 63, 64] {
            let id = "a".repeat(length);
            let input = format!("{{{{{}}}}}", id); // Changed to double braces
            let result = parse_argument(&input);
            assert!(result.is_ok(), "{} character identifier should work. Error: {:?}", length, result.err());
        }

        for length in [65, 100, 1000] {
            let id = "a".repeat(length);
            let input = format!("{{{{{}}}}}", id); // Changed to double braces
            let result = parse_argument(&input);
            assert!(result.is_err(), "{} character identifier should fail", length);
        }
    }

}