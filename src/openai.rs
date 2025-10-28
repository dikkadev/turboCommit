#![allow(dead_code)]

use colored::Colorize;
use crossterm::cursor::{MoveToColumn, MoveToPreviousLine};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{execute, terminal};
use futures::StreamExt;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use std::{fmt, process};

use crate::animation;
use crate::util::count_lines;

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
#[serde(untagged)]
pub enum Request {
    Standard(StandardRequest),
    OSeries(OSeriesRequest),
}

#[derive(Debug, Serialize)]
pub struct StandardRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub n: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    pub frequency_penalty: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    stream: bool,
}

#[derive(Debug, Serialize)]
pub struct OSeriesRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub n: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    stream: bool,
}

impl Request {
    pub fn new(
        model: String,
        messages: Vec<Message>,
        n: i32,
        temperature: f64,
        frequency_penalty: f64,
    ) -> Self {
        if model.starts_with("o1") || model.starts_with("o3") || model.starts_with("gpt-5") {
            Self::OSeries(OSeriesRequest {
                model,
                messages,
                n,
                reasoning_effort: None,
                stream: true,
            })
        } else {
            Self::Standard(StandardRequest {
                model,
                messages,
                n,
                temperature: if temperature == 0.0 { None } else { Some(temperature) },
                frequency_penalty,
                reasoning_effort: None,
                stream: true,
            })
        }
    }

    pub fn with_reasoning_effort(self, effort: Option<String>) -> Self {
        match self {
            Self::Standard(mut req) => {
                req.reasoning_effort = effort;
                Self::Standard(req)
            }
            Self::OSeries(mut req) => {
                req.reasoning_effort = effort;
                Self::OSeries(req)
            }
        }
    }

    fn model(&self) -> &str {
        match self {
            Self::Standard(req) => &req.model,
            Self::OSeries(req) => &req.model,
        }
    }

    fn n(&self) -> i32 {
        match self {
            Self::Standard(req) => req.n,
            Self::OSeries(req) => req.n,
        }
    }

    pub async fn execute(
        &self,
        api_key: String,
        no_animations: bool,
        prompt_tokens: usize,
        api_endpoint: String,
        debug: bool,
        debug_logger: &mut crate::debug_log::DebugLogger,
    ) -> anyhow::Result<Vec<String>> {
        let mut choices = vec![String::new(); self.n() as usize];
        let json = serde_json::to_string(self)?;

        // First make a regular request to check if it will be accepted
        let client = reqwest::Client::new();
        let response = client
            .post(&api_endpoint)
            .header("Content-Type", "application/json")
            .bearer_auth(&api_key)
            .body(json.clone())
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_body = response.text().await?;
            
            // Try to parse as OpenAI error
            let error_details = match serde_json::from_str::<ErrorRoot>(&error_body) {
                Ok(error_root) => format!(
                    "OpenAI Error:\n  Type: {}\n  Message: {}\n  Code: {:?}\n  Parameter: {:?}\n\nFull Response:\n{}",
                    error_root.error.type_field,
                    error_root.error.message,
                    error_root.error.code,
                    error_root.error.param,
                    error_body
                ),
                Err(_) => format!("Raw Response:\n{}", error_body),
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

        let loading_ai_animation = animation::start(
            String::from("Asking AI..."),
            no_animations || debug,
            std::io::stdout(),
        )
        .await;

        let request_builder = client
            .post(api_endpoint.clone())
            .header("Content-Type", "application/json")
            .bearer_auth(api_key)
            .body(json);

        let term_width = terminal::size()?.0 as usize;
        let mut stdout = std::io::stdout();
        let mut es = EventSource::new(request_builder)?;
        let mut lines_to_move_up = 0;
        let mut response_tokens = 0;

        // Only show minimal info in regular debug mode
        if debug && !no_animations {
            println!("\n{}", "Request Info:".blue().bold());
            println!("  Model: {}", self.model().purple());
            println!("  API: {}", api_endpoint.purple());
            println!("  Input tokens: {}", prompt_tokens.to_string().purple());
        }

        while let Some(event) = es.next().await {
            if no_animations || debug {
                match event {
                    Ok(Event::Message(message)) => {
                        if message.data == "[DONE]" {
                            break;
                        }
                        let resp = serde_json::from_str::<Response>(&message.data)
                            .map_or_else(|_| Response::default(), |r| r);
                        response_tokens += 1;
                        for choice in resp.choices {
                            if let Some(content) = choice.delta.content {
                                choices[choice.index as usize].push_str(&content);
                            }
                        }
                    }
                    Err(e) => {
                        // The error string from reqwest_eventsource includes the full response
                        let error_str = e.to_string();
                        let error_details = if let Some(error_json) = error_str.strip_prefix("Error response: ") {
                            // Try to parse as OpenAI error format
                            match serde_json::from_str::<ErrorRoot>(error_json) {
                                Ok(error_root) => format!(
                                    "OpenAI Error:\n  Type: {}\n  Message: {}\n  Code: {:?}\n\nFull Response:\n{}",
                                    error_root.error.type_field,
                                    error_root.error.message,
                                    error_root.error.code,
                                    error_json
                                ),
                                Err(_) => format!("Raw Response:\n{}", error_json)
                            }
                        } else {
                            format!("Error: {}", error_str)
                        };

                        let error_msg = format!(
                            "API request failed:\nEndpoint: {}\n\n{}",
                            api_endpoint, error_details
                        );
                        debug_logger.log_error(&error_msg);
                        println!("{}", "API Error:".red().bold());
                        println!("{}", error_msg);
                        process::exit(1);
                    }
                    _ => {}
                }
            } else {
                if !loading_ai_animation.is_finished() {
                    loading_ai_animation.abort();
                    execute!(
                        std::io::stdout(),
                        Clear(ClearType::CurrentLine),
                        MoveToColumn(0),
                    )?;
                    print!("\n\n")
                }
                match event {
                    Ok(Event::Message(message)) => {
                        if message.data == "[DONE]" {
                            break;
                        }
                        execute!(stdout, MoveToPreviousLine(lines_to_move_up),)?;
                        lines_to_move_up = 0;
                        execute!(stdout, Clear(ClearType::FromCursorDown),)?;
                        let resp = serde_json::from_str::<Response>(&message.data)
                            .map_or_else(|_| Response::default(), |r| r);
                        response_tokens += 1;
                        for choice in resp.choices {
                            if let Some(content) = choice.delta.content {
                                choices[choice.index as usize].push_str(&content);
                            }
                        }
                        for (i, choice) in choices.iter().enumerate() {
                            let outp = format!(
                                "{}{}\n{}\n",
                                if i == 0 {
                                    format!(
                                        "Tokens used: {} input, {} output\n",
                                        crate::util::format_token_count(prompt_tokens).purple(),
                                        crate::util::format_token_count(response_tokens).purple(),
                                    )
                                    .bright_black()
                                } else {
                                    "".bright_black()
                                },
                                format!("[{}]====================", format!("{i}").purple())
                                    .bright_black(),
                                choice,
                            );
                            print!("{outp}");
                            lines_to_move_up += count_lines(&outp, term_width) - 1;
                        }
                    }
                    Err(e) => {
                        println!("{e}");
                        process::exit(1);
                    }
                    _ => {}
                }
            }
        }

        if no_animations || debug {
            println!(
                "Tokens: {} in, {} out (total: {})",
                crate::util::format_token_count(prompt_tokens).purple(),
                crate::util::format_token_count(response_tokens).purple(),
                crate::util::format_token_count(prompt_tokens + response_tokens).purple(),
            );
            for (i, choice) in choices.iter().enumerate() {
                println!(
                    "[{}] {}\n{}\n",
                    format!("{i}").purple(),
                    "=".repeat(77 - i.to_string().len()),
                    choice
                );
            }
        } else {
            // For regular mode (non-debug), show the final messages nicely formatted
            // Only show the messages header if we have multiple choices
            if choices.len() > 1 {
                println!("\n{}", "Generated Commit Messages:".blue().bold());
            }
            // Don't show the messages here if it's a reasoning response (has <think> tag)
            // as it will be handled by process_response
            if !choices[0].contains("<think>") {
                for (i, choice) in choices.iter().enumerate() {
                    println!(
                        "[{}] {}\n{}",
                        format!("{i}").purple(),
                        "=".repeat(77 - i.to_string().len()),
                        choice
                    );
                }
            }
        }

        execute!(
            stdout,
            Print(format!("{}\n", "=======================".bright_black())),
        )?;

        execute!(
            stdout,
            MoveToPreviousLine(lines_to_move_up),
            Clear(ClearType::FromCursorDown),
        )?;

        Ok(choices)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Choice {
    pub index: i64,
    pub finish_reason: Option<String>,
    pub delta: Delta,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Delta {
    pub role: Option<Role>,
    pub content: Option<String>,
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
    fn test_temperature_disabled_when_zero() {
        let request = Request::new(
            "gpt-4".to_string(),
            vec![Message::user("test".to_string())],
            1,
            0.0,
            0.0,
        );

        match request {
            Request::Standard(req) => {
                assert_eq!(req.temperature, None, "Temperature should be None when set to 0.0");
            }
            _ => panic!("Expected StandardRequest"),
        }
    }

    #[test]
    fn test_temperature_included_when_nonzero() {
        let request = Request::new(
            "gpt-4".to_string(),
            vec![Message::user("test".to_string())],
            1,
            1.0,
            0.0,
        );

        match request {
            Request::Standard(req) => {
                assert_eq!(req.temperature, Some(1.0), "Temperature should be Some(1.0) when set to 1.0");
            }
            _ => panic!("Expected StandardRequest"),
        }
    }

    #[test]
    fn test_o_series_models_use_oseries_request() {
        let request = Request::new(
            "o1".to_string(),
            vec![Message::user("test".to_string())],
            1,
            1.0,
            0.0,
        );

        match request {
            Request::OSeries(_) => {
                // Success - o1 should use OSeries request
            }
            _ => panic!("Expected OSeriesRequest for o1 model"),
        }
    }

    #[test]
    fn test_gpt5_models_use_oseries_request() {
        // Test all GPT-5 variants use OSeries (no temperature support)
        let models = vec!["gpt-5", "gpt-5-nano", "gpt-5-mini", "gpt-5-codex"];
        
        for model_name in models {
            let request = Request::new(
                model_name.to_string(),
                vec![Message::user("test".to_string())],
                1,
                1.0,
                0.0,
            );

            match request {
                Request::OSeries(_) => {
                    // Success - GPT-5 models should use OSeries request
                }
                _ => panic!("Expected OSeriesRequest for {} model", model_name),
            }
        }
    }

    #[test]
    fn test_temperature_serialization_skipped_when_none() {
        let request = Request::new(
            "gpt-4".to_string(),
            vec![Message::user("test".to_string())],
            1,
            0.0,
            0.0,
        );

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        assert!(!json.contains("temperature"), "Serialized JSON should not contain 'temperature' field when it's None");
    }

    #[test]
    fn test_temperature_serialization_included_when_some() {
        let request = Request::new(
            "gpt-4".to_string(),
            vec![Message::user("test".to_string())],
            1,
            1.5,
            0.0,
        );

        let json = serde_json::to_string(&request).expect("Failed to serialize");
        assert!(json.contains("temperature"), "Serialized JSON should contain 'temperature' field when it's Some");
        assert!(json.contains("1.5"), "Serialized JSON should contain the temperature value");
    }
}
