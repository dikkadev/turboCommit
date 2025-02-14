use actor::Actor;
use colored::Colorize;
use config::Config;

use openai::Message;

use std::{env, process, time::Duration};

mod actor;
mod animation;
mod cli;
mod config;
mod git;
mod model;
mod openai;
mod util;
mod debug_log;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // First get the default config just to parse CLI options
    let default_config = Config::load()?;
    let mut options = cli::Options::new(env::args(), &default_config);

    // If check_version_only is set, just check version and exit
    if options.check_version_only {
        util::check_version().await;
        return Ok(());
    }

    // Load the actual config we'll use (either custom or default)
    let config = if let Some(config_path) = options.config_file.as_ref() {
        Config::load_from_path(std::path::Path::new(config_path))?
    } else {
        default_config
    };

    // Update options with the final config values
    options = cli::Options::new(env::args(), &config);

    let api_key = match &options.api_key {
        Some(ref key) => key.clone(),
        None => {
            let env_var = &config.api_key_env_var;
            if env_var.trim().is_empty() {
                // If env_var is empty, no API key is needed
                String::new()
            } else {
                // Only check environment variable if env_var is not empty
                match env::var(env_var) {
                    Ok(key) => key,
                    Err(_) => {
                        println!("{}", format!("No API key found. Either:").red());
                        println!("  1. Set the {} environment variable", env_var.purple());
                        println!("  2. Use the {} option", "--api-key <key>".purple());
                        println!("\n{}", "For API key safety best practices, see: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety".bright_black());
                        process::exit(1);
                    }
                }
            }
        }
    };

    let mut actor = Actor::new(
        options.clone(),
        api_key,
        options.api_endpoint.clone(),
    );

    let repo = git::get_repo()?;

    let system_len = openai::count_token(options.system_msg.as_ref().unwrap_or(&config.system_msg)).unwrap_or(0);
    let extra_len = openai::count_token(&options.msg).unwrap_or(0);

    // Add system message first
    actor.add_message(Message::system(options.system_msg.unwrap_or(config.system_msg.clone())));

    // Handle amend mode
    if options.amend {
        // When amending, we don't want any staged files
        if git::has_staged_changes(&repo)? {
            println!("{}", "Error: You have staged changes.".red());
            println!("{}", "When using --amend, you should not have any staged changes.".bright_black());
            println!("{}", "The --amend option only changes the commit message of the last commit.".bright_black());
            println!("{}", "If you want to include new changes, either:".bright_black());
            println!("{}", "1. Commit them first normally, then amend that commit".bright_black());
            println!("{}", "2. Or use git commit --amend manually to include them".bright_black());
            process::exit(1);
        }

        // Get the diff from the last commit
        let diff = git::get_last_commit_diff(&repo)?;
        if diff.is_empty() {
            println!("{}", "Error: Could not get changes from the last commit.".red());
            println!("{}", "Make sure you have at least one commit in your repository.".bright_black());
            process::exit(1);
        }
        actor.add_message(Message::user(diff));
        actor.used_tokens = system_len + extra_len;
    } else {
        // Normal commit mode - get diff from staged changes
        let (diff, diff_tokens) = util::decide_diff(&repo, system_len + extra_len, options.model.context_size(), options.always_select_files)?;
        actor.add_message(Message::user(diff));
        actor.used_tokens = system_len + extra_len + diff_tokens;
    }

    // Add any extra message from command line
    if !options.msg.is_empty() {
        actor.add_message(Message::user(options.msg));
    }

    if options.auto_commmit {
        let _ = actor.auto_commit().await?;
    } else {
        let _ = actor.start().await;
    }

    // Only check for updates if not disabled in config or CLI
    if !options.disable_auto_update_check {
        util::check_version().await;
    }

    if util::check_config_age(Duration::from_secs(60 * 60 * 24 * 30 * 6)) {
        if !util::is_system_prompt_same_as_default(&config.system_msg) {
            println!(
                "\n{}\n{}\n{}",
                "Your system prompt seems to be old.".yellow(),
                "There is a new default recommended system prompt. To apply it, delete the `system_msg` field in your config file.".bright_black(),
                "To get rid of this message, simply save your config file to change the last modified date.".bright_black()
            );
        }
    }

    Ok(())
}
