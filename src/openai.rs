#![allow(dead_code)]

use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fmt, process};

use crate::debug_log::DebugLogger;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Developer,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Developer => write!(f, "developer"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub const fn system(content: String) -> Self {
        Self {
            role: Role::System,
            content,
        }
    }
    pub const fn developer(content: String) -> Self {
        Self {
            role: Role::Developer,
            content,
        }
    }
    pub const fn user(content: String) -> Self {
        Self {
            role: Role::User,
            content,
        }
    }
    pub const fn assistant(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommitSuggestion {
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
}

impl CommitSuggestion {
    pub fn as_commit_message(&self) -> String {
        let title = self.title.trim();
        match self
            .body
            .as_ref()
            .map(|b| b.trim())
            .filter(|b| !b.is_empty())
        {
            Some(body) => format!("{title}\n\n{body}"),
            None => title.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitSuggestionsEnvelope {
    pub suggestions: Vec<CommitSuggestion>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ErrorRoot {
    pub error: Error,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub message: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub param: Option<String>,
    pub code: Option<String>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({:?}): {:?}",
            self.type_field.red(),
            self.code,
            self.message
        )
    }
}

#[derive(Debug, Serialize)]
pub struct Request {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip)]
    suggestion_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbosity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Serialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    type_field: String,
    json_schema: JsonSchemaFormat,
}

#[derive(Debug, Serialize)]
pub struct JsonSchemaFormat {
    name: String,
    strict: bool,
    schema: Value,
}

impl ResponseFormat {
    fn commit_suggestions(suggestion_count: usize) -> Self {
        let count = suggestion_count.max(1) as u64;
        Self {
            type_field: "json_schema".to_string(),
            json_schema: JsonSchemaFormat {
                name: "commit_suggestions".to_string(),
                strict: true,
                schema: json!({
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "suggestions": {
                            "type": "array",
                            "minItems": count,
                            "maxItems": count,
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "properties": {
                                    "title": {
                                        "type": "string",
                        "description": "Conventional commit title (<type>(scope?): description)",
                                        "minLength": 1
                                    },
                                    "body": {
                                        "type": ["string", "null"],
                                        "description": "Optional conventional commit body paragraph focusing on motivation"
                                    }
                                },
                                "required": ["title"]
                            }
                        }
                    },
                    "required": ["suggestions"]
                }),
            },
        }
    }
}

impl Request {
    pub fn new(model: String, messages: Vec<Message>, suggestion_count: usize) -> Self {
        let normalized = suggestion_count.max(1);
        Self {
            model,
            messages,
            suggestion_count: normalized,
            reasoning_effort: None,
            verbosity: None,
            response_format: Some(ResponseFormat::commit_suggestions(normalized)),
        }
    }

    pub fn with_reasoning_effort(mut self, effort: Option<String>) -> Self {
        self.reasoning_effort = effort;
        self
    }

    pub fn with_verbosity(mut self, verbosity: Option<String>) -> Self {
        self.verbosity = verbosity;
        self
    }

    pub fn suggestion_count(&self) -> usize {
        self.suggestion_count
    }

    pub async fn execute(
        &self,
        api_key: String,
        prompt_tokens: usize,
        api_endpoint: String,
        debug: bool,
        debug_logger: &mut DebugLogger,
    ) -> anyhow::Result<CompletionResult> {
        let client = reqwest::Client::new();
        let response = client
            .post(&api_endpoint)
            .header("Content-Type", "application/json")
            .bearer_auth(&api_key)
            .json(self)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            let error_details = match serde_json::from_str::<ErrorRoot>(&body) {
                Ok(error_root) => format!(
                    "OpenAI Error:\n  Type: {}\n  Message: {}\n  Code: {:?}\n  Parameter: {:?}\n\nFull Response:\n{}",
                    error_root.error.type_field,
                    error_root.error.message,
                    error_root.error.code,
                    error_root.error.param,
                    body
                ),
                Err(_) => format!("Raw Response:\n{}", body),
            };

            let error_msg = format!(
                "API request failed (HTTP {}):\nEndpoint: {}\n\n{}",
                status, api_endpoint, error_details
            );
            debug_logger.log_error(&error_msg);
            println!("{}", "API Error:".red().bold());
            println!("{}", error_msg);
            process::exit(1);
        }

        debug_logger.log_response(&body);

        let completion: ChatCompletionResponse = serde_json::from_str(&body).map_err(|err| {
            let msg = format!("Failed to parse API response as chat completion JSON: {err}");
            debug_logger.log_error(&format!("{msg}\nRaw body: {body}"));
            anyhow::anyhow!(msg)
        })?;

        let choice = completion
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("API response did not include any choices"))?;

        let structured_payload = choice
            .message
            .into_text()
            .ok_or_else(|| anyhow::anyhow!("Assistant response did not include textual content"))?;

        let envelope: CommitSuggestionsEnvelope = serde_json::from_str(&structured_payload)
            .map_err(|err| {
                let msg = format!("Failed to parse structured suggestions: {err}");
                debug_logger.log_error(&format!("{msg}\nPayload: {structured_payload}"));
                anyhow::anyhow!(msg)
            })?;

        if envelope.suggestions.is_empty() {
            return Err(anyhow::anyhow!(
                "Model returned zero commit suggestions; expected at least one"
            ));
        }

        if envelope.suggestions.len() != self.suggestion_count {
            println!(
                "{} {} -> {}",
                "Warning:".yellow(),
                "Model returned a different number of suggestions than requested".bright_black(),
                envelope.suggestions.len()
            );
        }

        if debug {
            println!("\n{}", "=== API Response ===".blue().bold());
            println!("  Model: {}", self.model.purple());
            println!("  Input tokens: {}", prompt_tokens.to_string().purple());
            if let Some(usage) = &completion.usage {
                println!(
                    "  Output tokens: {} (total: {})",
                    usage.completion_tokens.to_string().purple(),
                    usage.total_tokens.to_string().purple()
                );
            }
            println!(
                "  Suggestions returned: {}",
                envelope.suggestions.len().to_string().purple()
            );
        }

        Ok(CompletionResult {
            suggestions: envelope.suggestions,
            usage: completion.usage,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResult {
    pub suggestions: Vec<CommitSuggestion>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    pub index: usize,
    pub message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    pub role: Role,
    pub content: MessageContent,
}

impl ChoiceMessage {
    fn into_text(self) -> Option<String> {
        match self.content {
            MessageContent::Text(s) => Some(s),
            MessageContent::Array(parts) => {
                let mut text = String::new();
                for part in parts {
                    if part.kind == "output_text" || part.kind == "text" {
                        if let Some(content) = part.text {
                            text.push_str(&content);
                        }
                    }
                }
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Array(Vec<ContentPart>),
}

#[derive(Debug, Deserialize)]
struct ContentPart {
    #[serde(rename = "type")]
    kind: String,
    text: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    #[serde(default)]
    pub completion_tokens_details: CompletionTokensDetails,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CompletionTokensDetails {
    pub reasoning_tokens: usize,
    pub accepted_prediction_tokens: usize,
    pub rejected_prediction_tokens: usize,
}

pub fn count_token(s: &str) -> anyhow::Result<usize> {
    let bpe = tiktoken_rs::cl100k_base()?;
    let tokens = bpe.encode_with_special_tokens(s);
    Ok(tokens.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verbosity_with_request() {
        let request = Request::new(
            "gpt-5.1".to_string(),
            vec![Message::user("test".to_string())],
            1,
        )
        .with_verbosity(Some("high".to_string()));

        assert_eq!(request.verbosity, Some("high".to_string()));
    }

    #[test]
    fn test_reasoning_effort_none() {
        let request = Request::new(
            "gpt-5.1".to_string(),
            vec![Message::user("test".to_string())],
            1,
        )
        .with_reasoning_effort(Some("none".to_string()));

        assert_eq!(request.reasoning_effort, Some("none".to_string()));
    }

    #[test]
    fn test_verbosity_serialization_skipped_when_none() {
        let request = Request::new(
            "gpt-5.1".to_string(),
            vec![Message::user("test".to_string())],
            1,
        );

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        assert!(
            !json.contains("\"verbosity\""),
            "Serialized JSON should not contain 'verbosity' field when it's None"
        );
    }

    #[test]
    fn test_verbosity_serialization_included_when_some() {
        let request = Request::new(
            "gpt-5.1".to_string(),
            vec![Message::user("test".to_string())],
            1,
        )
        .with_verbosity(Some("high".to_string()));

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        assert!(
            json.contains("\"verbosity\""),
            "Serialized JSON should contain 'verbosity' field when it's Some"
        );
        assert!(
            json.contains("\"high\""),
            "Serialized JSON should contain the verbosity value"
        );
    }

    #[test]
    fn commit_suggestion_to_message_body_optional() {
        let suggestion = CommitSuggestion {
            title: "feat: example".to_string(),
            body: Some("Explain why".to_string()),
        };
        assert_eq!(
            suggestion.as_commit_message(),
            "feat: example\n\nExplain why".to_string()
        );

        let suggestion_no_body = CommitSuggestion {
            title: "fix: bug".to_string(),
            body: None,
        };
        assert_eq!(suggestion_no_body.as_commit_message(), "fix: bug");
    }
}
