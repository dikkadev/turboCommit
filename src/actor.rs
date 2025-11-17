use std::process;

use colored::Colorize;
use crossterm::execute;
use crossterm::style::Print;
use inquire::Select;
use edit;

use crate::cli::Options;
use crate::{git, jj, openai, util, debug_log::DebugLogger};

pub struct Actor {
    messages: Vec<openai::Message>,
    options: Options,
    api_key: String,
    pub used_tokens: usize,
    api_endpoint: String,
    debug_logger: DebugLogger,
    vcs_type: jj::VcsType,
}

impl Actor {
    pub fn new(options: Options, api_key: String, api_endpoint: String, vcs_type: jj::VcsType) -> Self {
        // Get debug_file before moving options
        let debug_file = options.debug_file.clone();
        Self {
            messages: Vec::new(),
            options,
            api_key,
            used_tokens: 0,
            api_endpoint,
            debug_logger: DebugLogger::new(debug_file),
            vcs_type,
        }
    }

    pub fn add_message(&mut self, message: openai::Message) {
        // Log message content if debug_context is enabled
        if self.options.debug_context {
            println!("\n{}", "=== Message to AI ===".blue().bold());
            println!("{}: {}", 
                format!("{:?}", message.role).purple().bold(),
                message.content.bright_black()
            );
            println!("{}", "=====================".blue().bold());
        }
        self.messages.push(message);
    }

    async fn ask(&mut self) -> anyhow::Result<Vec<String>> {
        let n = if self.options.enable_reasoning { 1 } else { self.options.n };
        let mut request = openai::Request::new(
            self.options.model.clone().to_string(),
            self.messages.clone(),
            n,
            self.options.t,
            self.options.f,
        );

        // Add reasoning effort if reasoning mode is enabled
        if self.options.enable_reasoning {
            request = request.with_reasoning_effort(self.options.reasoning_effort.clone());
        }

        // Add verbosity if specified
        if let Some(ref verbosity) = self.options.verbosity {
            request = request.with_verbosity(Some(verbosity.clone()));
        }

        // Log request details
        let json = serde_json::to_string(&request)?;
        self.debug_logger.log_request(&json);

        // Log basic info about the request
        let info = format!(
            "model={}, reasoning={}, effort={}, verbosity={}, messages={}, tokens={}",
            self.options.model.0,
            self.options.enable_reasoning,
            self.options.reasoning_effort.as_deref().unwrap_or("none"),
            self.options.verbosity.as_deref().unwrap_or("default"),
            self.messages.len(),
            self.used_tokens
        );
        self.debug_logger.log_info(&info);

        // Only show minimal info in regular debug mode
        if self.options.debug && self.options.debug_file.is_none() {
            println!("\n{}", "Request Info:".blue().bold());
            println!("  Model: {}", self.options.model.0.purple());
            if self.options.enable_reasoning {
                println!("  Reasoning: {} ({})", 
                    "enabled".purple(),
                    self.options.reasoning_effort.as_deref().unwrap_or("medium").purple()
                );
            }
            println!("  Messages: {}", self.messages.len().to_string().purple());
            println!("  Tokens (input): {}", self.used_tokens.to_string().purple());
        }

        match request
            .execute(
                self.api_key.clone(),
                self.options.print_once,
                self.used_tokens,
                self.api_endpoint.clone(),
                self.options.debug,
                &mut self.debug_logger,
            )
            .await
        {
            Ok(choices) => {
                // Log successful response
                self.debug_logger.log_response(&format!(
                    "success: generated {} choices",
                    choices.len()
                ));
                Ok(choices)
            }
            Err(e) => {
                // Log error details
                self.debug_logger.log_error(&format!("API error: {:#?}", e));
                Err(e)
            }
        }
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
                    match self.vcs_type {
                        jj::VcsType::Git => {
                            match git::commit(message, self.options.amend) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("{e}");
                                    process::exit(1);
                                }
                            };
                            println!("{} 🎉", if self.options.amend { "Commit message amended!" } else { "Commit successful!" }.purple());
                        }
                        jj::VcsType::Jujutsu => {
                            match jj::set_jj_description(self.options.jj_revision.as_deref(), &message) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("{e}");
                                    process::exit(1);
                                }
                            };
                            println!("{} 🎉", "Description set successfully!".purple());
                        }
                    }
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
        
        match self.vcs_type {
            jj::VcsType::Git => {
                git::commit(message.clone(), self.options.amend)?;
            }
            jj::VcsType::Jujutsu => {
                jj::set_jj_description(self.options.jj_revision.as_deref(), &message)?;
            }
        }
        
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
