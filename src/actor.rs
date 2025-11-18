use std::process;

use colored::Colorize;
use crossterm::execute;
use crossterm::style::Print;
use edit;
use inquire::Select;

use crate::cli::Options;
use crate::{debug_log::DebugLogger, git, jj, openai, util};

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
    pub fn new(
        options: Options,
        api_key: String,
        api_endpoint: String,
        vcs_type: jj::VcsType,
    ) -> Self {
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

    fn print_suggestions(
        &self,
        suggestions: &[openai::CommitSuggestion],
        usage: Option<&openai::Usage>,
    ) {
        if suggestions.is_empty() {
            return;
        }

        println!("\n{}", "Generated Commit Messages:".blue().bold());
        if let Some(usage) = usage {
            println!(
                "{} {} in, {} out (total: {})",
                "Tokens:".bright_black(),
                util::format_token_count(usage.prompt_tokens).purple(),
                util::format_token_count(usage.completion_tokens).purple(),
                util::format_token_count(usage.total_tokens).purple(),
            );
        } else {
            println!(
                "{} {} in (model usage unavailable)",
                "Tokens:".bright_black(),
                util::format_token_count(self.used_tokens).purple()
            );
        }

        for (i, suggestion) in suggestions.iter().enumerate() {
            println!(
                "[{}] {}\n{}\n",
                format!("{i}").purple(),
                "=".repeat(77 - i.to_string().len()),
                suggestion.as_commit_message()
            );
        }
    }

    pub fn add_message(&mut self, message: openai::Message) {
        // Log message content if debug_context is enabled
        if self.options.debug_context {
            println!("\n{}", "=== Message to AI ===".blue().bold());
            println!(
                "{}: {}",
                format!("{:?}", message.role).purple().bold(),
                message.content.bright_black()
            );
            println!("{}", "=====================".blue().bold());
        }
        self.messages.push(message);
    }

    async fn ask(&mut self) -> anyhow::Result<openai::CompletionResult> {
        let suggestion_count = self.options.n.max(1) as usize;
        let mut request = openai::Request::new(
            self.options.model.clone().to_string(),
            self.messages.clone(),
            suggestion_count,
        );

        // Add reasoning effort (default from config or CLI override)
        if let Some(ref effort) = self.options.reasoning_effort {
            request = request.with_reasoning_effort(Some(effort.clone()));
        }

        // Add verbosity (default from config or CLI override)
        if let Some(ref verbosity) = self.options.verbosity {
            request = request.with_verbosity(Some(verbosity.clone()));
        }

        // Log request details
        let json = serde_json::to_string(&request)?;
        self.debug_logger.log_request(&json);

        // Log basic info about the request
        let info = format!(
            "model={}, effort={}, verbosity={}, messages={}, tokens={}",
            self.options.model.0,
            self.options
                .reasoning_effort
                .as_deref()
                .unwrap_or("default"),
            self.options.verbosity.as_deref().unwrap_or("default"),
            self.messages.len(),
            self.used_tokens
        );
        self.debug_logger.log_info(&info);

        // Show useful info in debug mode
        if self.options.debug && self.options.debug_file.is_none() {
            println!("\n{}", "=== Request Info ===".blue().bold());
            println!(
                "  {}: {}",
                "Model".bright_black(),
                self.options.model.0.purple()
            );
            println!(
                "  {}: {}",
                "Reasoning Effort".bright_black(),
                self.options
                    .reasoning_effort
                    .as_deref()
                    .unwrap_or("default")
                    .purple()
            );
            println!(
                "  {}: {}",
                "Verbosity".bright_black(),
                self.options
                    .verbosity
                    .as_deref()
                    .unwrap_or("default")
                    .purple()
            );
            println!(
                "  {}: {}",
                "Messages".bright_black(),
                self.messages.len().to_string().purple()
            );
            println!(
                "  {}: {}",
                "Input Tokens".bright_black(),
                self.used_tokens.to_string().purple()
            );
        }

        match request
            .execute(
                self.api_key.clone(),
                self.used_tokens,
                self.api_endpoint.clone(),
                self.options.debug,
                &mut self.debug_logger,
            )
            .await
        {
            Ok(result) => {
                self.debug_logger.log_response(&format!(
                    "success: generated {} suggestions",
                    result.suggestions.len()
                ));
                self.print_suggestions(&result.suggestions, result.usage.as_ref());
                Ok(result)
            }
            Err(e) => {
                // Log error details
                self.debug_logger.log_error(&format!("API error: {:#?}", e));
                Err(e)
            }
        }
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        if self.options.debug {
            println!("\n{}", "=== Starting Commit Generation ===".blue().bold());
        }

        let suggestions = self.ask().await?.suggestions;

        let formatted_first_choices: Vec<String> = suggestions
            .iter()
            .map(openai::CommitSuggestion::as_commit_message)
            .collect();

        if formatted_first_choices.is_empty() {
            return Err(anyhow::anyhow!("No commit suggestions were generated"));
        }

        let mut message = match util::choose_message(formatted_first_choices) {
            Some(message) => message,
            None => {
                if self.options.debug {
                    println!("{}", "User cancelled message selection".yellow());
                }
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
                    if self.options.debug {
                        println!("\n{}", "=== Committing ===".blue().bold());
                    }
                    match self.vcs_type {
                        jj::VcsType::Git => {
                            match git::commit(message, self.options.amend) {
                                Ok(_) => {}
                                Err(e) => {
                                    println!("{e}");
                                    process::exit(1);
                                }
                            };
                            println!(
                                "{} 🎉",
                                if self.options.amend {
                                    "Commit message amended!"
                                } else {
                                    "Commit successful!"
                                }
                                .purple()
                            );
                        }
                        jj::VcsType::Jujutsu => {
                            match jj::set_jj_description(
                                self.options.jj_revision.as_deref(),
                                &message,
                            ) {
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
                    if self.options.debug {
                        println!("\n{}", "=== Opening Editor ===".blue().bold());
                    }
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
                    if self.options.debug {
                        println!("\n{}", "=== Revising Message ===".blue().bold());
                    }
                    self.add_message(openai::Message::assistant(message.clone()));
                    let input = inquire::Text::new("Revise:").prompt()?;
                    if self.options.debug {
                        println!("  User input: {}", input.bright_black());
                    }
                    self.add_message(openai::Message::user(input));

                    let completion = self.ask().await?;
                    let revision_choices: Vec<String> = completion
                        .suggestions
                        .iter()
                        .map(openai::CommitSuggestion::as_commit_message)
                        .collect();

                    if revision_choices.is_empty() {
                        println!("{}", "No revision suggestions produced".yellow());
                        continue;
                    }

                    message = match util::choose_message(revision_choices) {
                        Some(message) => message,
                        None => {
                            if self.options.debug {
                                println!("{}", "User cancelled message selection".yellow());
                            }
                            return Ok(());
                        }
                    };
                }
                Task::Abort => {
                    if self.options.debug {
                        println!("\n{}", "=== Aborted ===".yellow().bold());
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn auto_commit(&mut self) -> anyhow::Result<String> {
        let completion = self.ask().await?;
        if completion.suggestions.is_empty() {
            return Err(anyhow::anyhow!("No commit message generated"));
        }
        let message = completion.suggestions[0].as_commit_message();

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
