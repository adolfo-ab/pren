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

#[derive(Debug, Clone)]
pub struct Prompt {
    pub name: String,
    pub template: PromptTemplate,
    pub tags: Vec<String>,
}
