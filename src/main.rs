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
    let config = Config::load()?;
    let options = cli::Options::new(env::args(), &config);

    // If check_version_only is set, just check version and exit
    if options.check_version_only {
        util::check_version().await;
        return Ok(());
    }

    let api_key = match &options.api_key {
        Some(ref key) => key.clone(),
        None => {
            let env_var = &config.api_key_env_var;
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
    };

    let mut actor = Actor::new(
        options.clone(),
        api_key,
        options.api_endpoint.clone(),
    );

    let repo = git::get_repo()?;

    let system_len = openai::count_token(options.system_msg.as_ref().unwrap_or(&config.system_msg)).unwrap_or(0);
    let extra_len = openai::count_token(&options.msg).unwrap_or(0);

    let (diff, diff_tokens) =
        util::decide_diff(&repo, system_len + extra_len, options.model.context_size())?;

    // Use developer role if reasoning mode is enabled, system role otherwise
    if options.enable_reasoning {
        actor.add_message(Message::developer(options.system_msg.unwrap_or(config.system_msg.clone())));
    } else {
        actor.add_message(Message::system(options.system_msg.unwrap_or(config.system_msg.clone())));
    }
    actor.add_message(Message::user(diff));

    if !options.msg.is_empty() {
        actor.add_message(Message::user(options.msg));
    }

    actor.used_tokens = system_len + extra_len + diff_tokens;

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
