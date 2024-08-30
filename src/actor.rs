use std::process;

use colored::Colorize;
use crossterm::execute;
use crossterm::style::Print;
use inquire::Select;

use crate::cli::Options;
use crate::{git, openai, util};

pub struct Actor {
    messages: Vec<openai::Message>,
    options: Options,
    api_key: String,
    pub used_tokens: usize,
}

impl Actor {
    pub fn new(options: Options, api_key: String) -> Self {
        Self {
            messages: Vec::new(),
            options,
            api_key,
            used_tokens: 0,
        }
    }

    pub fn add_message(&mut self, message: openai::Message) {
        self.messages.push(message);
    }

    async fn ask(&self) -> anyhow::Result<Vec<String>> {
        Ok(openai::Request::new(
            self.options.model.clone().to_string(),
            self.messages.clone(),
            self.options.n,
            self.options.t,
            self.options.f,
        )
        .execute(
            self.api_key.clone(),
            self.options.print_once,
            self.options.model.clone(),
            self.used_tokens,
        )
        .await?)
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        let first_choices = self.ask().await?;
        let mut message = match util::choose_message(first_choices) {
            Some(message) => message,
            None => {
                return Ok(());
            }
        };
        let tasks = vec![
            Task::Commit.to_str(),
            Task::Edit.to_str(),
            Task::Revise.to_str(),
            Task::Abort.to_str(),
        ];

        loop {
            let task = Select::new("What to do with the message?", tasks.clone()).prompt()?;

            match Task::from_str(task) {
                Task::Commit => {
                    match git::commit(message) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("{e}");
                            process::exit(1);
                        }
                    };
                    println!("{} 🎉", "Commit successful!".purple());
                    break;
                }
                Task::Edit => {
                    message = edit::edit(message)?;
                    execute!(
                        std::io::stdout(),
                        Print(format!(
                            "{}\n",
                            format!("[{}]=======", "Edited Message".purple()).bright_black()
                        )),
                        Print(&message),
                        Print(format!("{}\n", "=======================".bright_black())),
                    )?;
                }
                Task::Revise => {
                    self.add_message(openai::Message::assistant(message.clone()));
                    let input = inquire::Text::new("Revise:").prompt()?;
                    self.add_message(openai::Message::user(input));

                    let choices = self.ask().await?;

                    message = match util::choose_message(choices) {
                        Some(message) => message,
                        None => {
                            return Ok(());
                        }
                    };
                }
                Task::Abort => {
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn auto_commit(&mut self) -> anyhow::Result<String> {
        let choices = self.ask().await?;
        if choices.is_empty() {
            return Err(anyhow::anyhow!("No commit message generated"));
        }
        let message = choices[0].clone();
        git::commit(message.clone())?;
        Ok(message)
    }
}

enum Task {
    Commit,
    Edit,
    Revise,
    Abort,
}

impl Task {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Commit it" => Self::Commit,
            "Edit it & Commit" => Self::Edit,
            "Revise" => Self::Revise,
            "Abort" => Self::Abort,
            _ => unreachable!(),
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            Self::Commit => "Commit it",
            Self::Edit => "Edit it & Commit",
            Self::Revise => "Revise",
            Self::Abort => "Abort",
        }
    }
}
