use std::fmt;
use colored::Colorize;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system<S: Into<String>>(content: S) -> Message {
        Message {
            role: Role::System,
            content: content.into(),
        }
    }
    pub fn user<S: Into<String>>(content: S) -> Message {
        Message {
            role: Role::User,
            content: content.into(),
        }
    }
    pub fn assistant<S: Into<String>>(content: S) -> Message {
        Message {
            role: Role::Assistant,
            content: content.into(),
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
        write!(f, "{} ({:?}): {:?}", self.type_field.red(), self.code, self.message)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub model: String,
    pub messages: Vec<Message>,
}

impl Request {
    pub fn new<S: Into<String>>(model: S, messages: Vec<Message>) -> Request {
        Request {
            model: model.into(),
            messages,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Choice {
    pub index: i64,
    pub finish_reason: String,
    pub message: Message,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}
