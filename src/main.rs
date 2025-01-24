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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load();
    match config.save_if_changed() {
        Ok(_) => (),
        Err(err) => {
            println!("{}", format!("Unable to write to config: {err}").red());
            process::exit(1);
        }
    }
    let options = cli::Options::new(env::args(), &config);

    let Ok(api_key) = env::var("OPENAI_API_KEY") else {
        println!("{} {}", "OPENAI_API_KEY not set.".red(), "Refer to step 3 here: https://help.openai.com/en/articles/5112595-best-practices-for-api-key-safety".bright_black());
        process::exit(1);
    };

    let mut actor = Actor::new(
        options.clone(),
        api_key,
        config.api_endpoint.clone(),
    );

    let repo = git::get_repo()?;

    let system_len = openai::count_token(&config.system_msg).unwrap_or(0);
    let extra_len = openai::count_token(&options.msg).unwrap_or(0);

    let (diff, diff_tokens) =
        util::decide_diff(&repo, system_len + extra_len, options.model.context_size())?;

    actor.add_message(Message::system(config.system_msg.clone()));
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

    util::check_version().await;

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
