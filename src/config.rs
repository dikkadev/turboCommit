use crate::model;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub model: model::Model,
    #[serde(default)]
    pub api_endpoint: String,
    #[serde(default)]
    pub default_temperature: f64,
    #[serde(default)]
    pub default_frequency_penalty: f64,
    #[serde(default)]
    pub default_number_of_choices: i32,
    #[serde(default)]
    pub disable_print_as_stream: bool,
    #[serde(default)]
    pub system_msg: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: model::Model("gpt-4o-mini".to_string()),
            api_endpoint: String::from("https://api.openai.com/v1/chat/completions"),
            default_temperature: 1.05,
            default_frequency_penalty: 0.0,
            default_number_of_choices: 3,
            disable_print_as_stream: false,
            system_msg: String::from("As an AI that only returns conventional commits, you will receive input from the user in the form of a git diff of all staged files. You CANNOT generate anything that is not a conventional commit and a commit message only has 1 head line and at most 1 body.
Make sure the body reads as a single brief message, NOT a list of bullets or multiple commits.
Do not format your response as markdown or similiar! You are simple and exclusively respond with a single commit message.
No yapping in the body (Very important). KISS principle. Make it better than most human-written commit messages, without being verbose. Avoid listing the changes in the body. The body should be a single paragraph that explains the context and the change.
The user may give you more specific instructions or extra information. Only include the motivation behind the commit of the user provides it as addional information. If the user does not provide the motivation, do not include it in the commit message.
The user may ask for revisions.
Ensure that all commits follow these guidelines

- Commits must start with a type, which is a noun like feat, fix, chore, etc., followed by an optional scope, an optional ! for breaking changes, and a required terminal colon and space
- Use feat for new features and fix for bug fixes
- You may provide a scope after a type. The scope should be a noun describing a section of the codebase, surrounded by parentheses
- After the type/scope prefix, include a short description of the code changes. This description should be followed immediately by a colon and a space
- You may provide a longer commit body after the short description. Body should start one blank line after the description and can consist of any number of newline-separated paragraphs

No yapping!"),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = home::home_dir().map_or_else(
            || {
                println!("{}", "Unable to find home directory.".red());
                process::exit(1);
            },
            |path| path.join(".turbocommit.yaml"),
        );
        match std::fs::read_to_string(path) {
            Ok(config) => match serde_yaml::from_str::<Self>(&config) {
                Ok(config) => { 
                    if config.system_msg.trim().is_empty() {
                        let mut config = config;
                        config.system_msg = Self::default().system_msg;
                        return config;
                    }
                    config
                },
                Err(err) => {
                    println!(
                        "{}\n{}",
                        format!("Unable to parse config file: {}", err).red(),
                        "Using default config.".bright_black()
                    );
                    Default::default()
                }
            },
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::NotFound => {
                        println!("{}", "Using default config.".bright_black());
                    }
                    _ => {
                        println!(
                            "{}\n{}",
                            format!("Unable to read config file: {}\n", err).red(),
                            "Using default config.".bright_black()
                        );
                    }
                }
                Default::default()
            }
        }
    }
    pub fn save_if_changed(&self) -> Result<(), std::io::Error> {
        let path = home::home_dir().map_or_else(
            || {
                println!("{}", "Unable to find home directory.".red());
                process::exit(1);
            },
            |path| path.join(".turbocommit.yaml"),
        );
        let config = match serde_yaml::to_string(self) {
            Ok(config) => config,
            Err(err) => {
                println!("{}", format!("Unable to serialize config: {}", err).red());
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Unable to serialize config",
                ));
            }
        };

        if let Ok(existing_config) = std::fs::read_to_string(&path) {
            if existing_config == config {
                return Ok(());
            }
        }

        std::fs::write(path, config)
    }
    pub fn path() -> std::path::PathBuf {
        home::home_dir().map_or_else(
            || {
                println!("{}", "Unable to find home directory.".red());
                process::exit(1);
            },
            |path| path.join(".turbocommit.yaml"),
        )
    }
}
