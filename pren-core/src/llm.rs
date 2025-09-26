use std::fmt::Debug;
use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionError, CompletionModelDyn, Message};
use rig::providers::openai::{Client};

pub async fn get_completions_content(
    api_key: &str,
    base_url: &str,
    model_name: &str,
    prompt: &str,
) -> Result<String, CompletionError> {
    let client = Client::builder(api_key)
        .base_url(base_url)
        .build()
        .unwrap();

    let model = client
        .completion_model(model_name)
        .completions_api();

    let response = model
        .completion_request(Message::from(prompt))
        .send()
        .await?;

    match response.choice.first() {
        AssistantContent::Text(t) => Ok(t.text.clone()),
        _ => Err(CompletionError::ResponseError(
            "Expected text response, but got tool call or reasoning".to_string()
        )),
    }
}