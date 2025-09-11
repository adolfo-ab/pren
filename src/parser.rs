use nom::branch::alt;
use nom::bytes::complete::{tag, take_until, take_while1};
use nom::character::complete::alphanumeric1;
use nom::combinator::{map, rest, verify};
use nom::IResult;
use nom::multi::many0;
use nom::Parser;
use nom::sequence::delimited;
use crate::prompt::{PromptTemplate, PromptTemplatePart};

pub fn parse_template(input: &str) -> IResult<&str, PromptTemplate> {
    map(many0(parse_element), |parts| PromptTemplate { parts }).parse(input)
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
    delimited(tag("{{"), alphanumeric1, tag("}}")).parse(input)
}

pub fn parse_prompt_reference(input: &str) -> IResult<&str, &str> {
    delimited(tag("{{prompt:"), identifier, tag("}}")).parse(input)
}

pub fn parse_escaped_literal(input: &str) -> IResult<&str, &str> {
    delimited(tag("{{{{"), take_until("}}}}"), tag("}}}}")).parse(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '-' || c == '_').parse(input)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literal_text() {
        let result = parse_literal_text("Hello {{pren}}!");
        assert_eq!(result, Ok(("{{pren}}!", "Hello ")));
    }

    #[test]
    fn test_parse_argument() {
        let result = parse_argument("{{topic}} is the subject");
        assert_eq!(result, Ok((" is the subject", "topic")));
    }

    #[test]
    fn test_parse_prompt_reference() {
        let result = parse_prompt_reference("{{prompt:basic_prompt}} is the prompt");
        assert_eq!(result, Ok((" is the prompt", "basic_prompt")));
    }

    #[test]
    fn test_parse_escaped_literal() {
        let result = parse_escaped_literal("{{{{hello world}}}} more text");
        assert_eq!(result, Ok((" more text", "hello world")));
    }

    #[test]
    fn test_parse_element_argument() {
        let result = parse_element("{{username}}");
        assert_eq!(result, Ok(("", PromptTemplatePart::Argument(String::from("username")))));
    }

    #[test]
    fn test_parse_element_prompt_reference() {
        let result = parse_element("{{prompt:username}}");
        assert_eq!(result, Ok(("", PromptTemplatePart::PromptReference(String::from("username")))));
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

}