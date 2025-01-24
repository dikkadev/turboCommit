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

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub model: String,
    pub messages: Vec<Message>,
    pub n: i32,
    pub temperature: f64,
    pub frequency_penalty: f64,
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
        Self {
            model,
            messages,
            n,
            temperature,
            frequency_penalty,
            stream: true,
        }
    }

    pub async fn execute(
        &self,
        api_key: String,
        no_animations: bool,
        prompt_tokens: usize,
        api_endpoint: String,
    ) -> anyhow::Result<Vec<String>> {
        let mut choices = vec![String::new(); self.n as usize];

        let loading_ai_animation = animation::start(
            String::from("Asking AI..."),
            no_animations,
            std::io::stdout(),
        )
        .await;

        let json = serde_json::to_string(self)?;

        let request_builder = reqwest::Client::new()
            .post(api_endpoint)
            .header("Content-Type", "application/json")
            .bearer_auth(api_key)
            .body(json);

        let term_width = terminal::size()?.0 as usize;

        let mut stdout = std::io::stdout();

        let mut es = EventSource::new(request_builder)?;
        let mut lines_to_move_up = 0;
        let mut response_tokens = 0;

        while let Some(event) = es.next().await {
            if no_animations {
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
                        println!("{e}");
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

        if no_animations {
            println!(
                "Tokens used: {} input, {} output\n",
                crate::util::format_token_count(prompt_tokens).purple(),
                crate::util::format_token_count(response_tokens).purple(),
            );
            for (i, choice) in choices.iter().enumerate() {
                println!(
                    "[{}]====================\n{}\n",
                    format!("{i}").purple(),
                    choice
                );
            }
        }

        execute!(
            stdout,
            Print(format!("{}\n", "=======================".bright_black())),
        )?;

        Ok(choices)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Usage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

pub fn count_token(s: &str) -> anyhow::Result<usize> {
    let bpe = tiktoken_rs::cl100k_base()?;
    let tokens = bpe.encode_with_special_tokens(s);
    Ok(tokens.len())
}
