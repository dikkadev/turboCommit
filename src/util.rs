use std::time::Duration;

use colored::Colorize;
use inquire::MultiSelect;
use unicode_segmentation::UnicodeSegmentation;

use crate::{config::Config, git, openai};

pub fn decide_diff(
    repo: &git2::Repository,
    used_tokens: usize,
    context: usize,
    always_select_files: bool,
) -> anyhow::Result<(String, usize)> {
    let staged_files = git::staged_files(&repo)?;
    let mut diff = git::diff(&repo, &staged_files)?;
    let mut diff_tokens = openai::count_token(&diff)?;

    if diff_tokens == 0 {
        println!(
            "{} {}",
            "No staged files.".red(),
            "Please stage the files you want to commit.".bright_black()
        );
        std::process::exit(1);
    }

    if always_select_files || used_tokens + diff_tokens > context {
        if always_select_files {
            println!(
                "{} {}",
                "File selection mode:".blue(),
                "Select the files you want to include in the commit.".bright_black()
            );
        } else {
            println!(
                "{} {}",
                "The request is too long!".red(),
                format!(
                    "The request is ~{} tokens long, while the maximum is {}.",
                    used_tokens + diff_tokens,
                    context
                )
                .bright_black()
            );
        }
        let selected_files = MultiSelect::new(
            "Select the files you want to include in the diff:",
            staged_files.clone(),
        )
        .prompt()?;
        diff = git::diff(&repo, &selected_files)?;
        diff_tokens = openai::count_token(&diff)?;
    }
    Ok((diff, diff_tokens))
}

#[must_use]
pub fn count_lines(text: &str, max_width: usize) -> u16 {
    if text.is_empty() {
        return 0;
    }
    let mut line_count = 0;
    let mut current_line_width = 0;
    for cluster in UnicodeSegmentation::graphemes(text, true) {
        match cluster {
            "\r" | "\u{FEFF}" => {}
            "\n" => {
                line_count += 1;
                current_line_width = 0;
            }
            _ => {
                current_line_width += 1;
                if current_line_width > max_width {
                    line_count += 1;
                    current_line_width = cluster.chars().count();
                }
            }
        }
    }

    line_count + 1
}

pub fn check_config_age(max_age: Duration) -> bool {
    let path = Config::path();
    let metadata = match std::fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return false;
        }
    };
    let last_modified = metadata.modified().unwrap();
    let now = std::time::SystemTime::now();
    match now.duration_since(last_modified) {
        Ok(duration) => duration > max_age,
        Err(_) => false,
    }
}

pub fn is_system_prompt_same_as_default(system_msg: &str) -> bool {
    let default = Config::default().system_msg;
    system_msg == default
}

pub async fn check_version() {
    let client = match crates_io_api::AsyncClient::new(
        "turbocommit latest version checker",
        Duration::from_millis(1000),
    ) {
        Ok(client) => client,
        Err(_) => {
            return;
        }
    };
    let turbo = match client.get_crate("turbocommit").await {
        Ok(turbo) => turbo,
        Err(_) => {
            return;
        }
    };
    let newest_version = turbo.versions[0].num.clone();
    let current_version = env!("CARGO_PKG_VERSION");

    if current_version != newest_version {
        println!(
            "\n{} {}",
            "New version available!".yellow(),
            format!("v{}", newest_version).purple()
        );
        println!(
            "To update, run\n{}",
            "cargo install --force turbocommit".purple()
        );
    }
}

pub fn choose_message(choices: Vec<String>) -> Option<String> {
    if choices.len() == 1 {
        return Some(choices[0].clone());
    }
    let max_index = choices.len();
    let commit_index = match inquire::CustomType::<usize>::new(&format!(
        "Which commit message do you want to use? {}",
        "<ESC> to cancel".bright_black()
    ))
    .with_validator(move |i: &usize| {
        if *i >= max_index {
            Err(inquire::CustomUserError::from("Invalid index"))
        } else {
            Ok(inquire::validator::Validation::Valid)
        }
    })
    .prompt()
    {
        Ok(index) => index,
        Err(_) => {
            return None;
        }
    };
    Some(choices[commit_index].clone())
}

pub fn format_token_count(tokens: usize) -> String {
    format!("{:.2}k", tokens as f64 / 1000.0)
}
