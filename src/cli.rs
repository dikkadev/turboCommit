use crate::config::Config;
use crate::model;
use crate::openai::count_token;
use colored::Colorize;
use std::str::FromStr;
use std::{cmp, env, process};

#[derive(Debug, Clone)]
pub struct Options {
    pub n: i32,
    pub msg: String,
    pub print_once: bool,
    pub model: model::Model,
    pub auto_commmit: bool,
    pub check_version_only: bool,
    pub api_endpoint: String,
    pub system_msg: Option<String>,
    pub disable_auto_update_check: bool,
    pub api_key: Option<String>,
    pub reasoning_effort: Option<String>,
    pub verbosity: Option<String>,
    pub debug: bool,
    pub debug_file: Option<String>,
    pub debug_context: bool,
    pub always_select_files: bool,
    pub config_file: Option<String>,
    pub amend: bool,
    // Jujutsu VCS specific options
    pub jj_revision: Option<String>,
    pub jj_rewrite: bool,
}

impl From<&Config> for Options {
    fn from(config: &Config) -> Self {
        Self {
            n: config.default_number_of_choices,
            msg: String::new(),
            print_once: config.disable_print_as_stream,
            model: config.model.clone(),
            auto_commmit: false,
            check_version_only: false,
            api_endpoint: config.api_endpoint.clone(),
            system_msg: None,
            disable_auto_update_check: config.disable_auto_update_check,
            api_key: None,
            reasoning_effort: Some(config.reasoning_effort.clone()),
            verbosity: Some(config.verbosity.clone()),
            debug: false,
            debug_file: None,
            debug_context: false,
            always_select_files: false,
            config_file: None,
            amend: false,
            jj_revision: None,
            jj_rewrite: config.jj_rewrite_default,
        }
    }
}

impl Options {
    pub fn new<I>(args: I, conf: &Config) -> Self
    where
        I: Iterator<Item = String>,
    {
        let mut opts = Self::from(conf);
        let mut iter = args.skip(1);
        let mut msg = String::new();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-n" => {
                    if let Some(n) = iter.next() {
                        opts.n = n.parse().map_or_else(
                            |_| {
                                println!(
                                    "{} {}",
                                    "Could not parse n.".red(),
                                    "Please enter an integer.".bright_black()
                                );
                                process::exit(1);
                            },
                            |n| cmp::max(1, n),
                        );
                    }
                }
                "-p" | "--print-once" => {
                    opts.print_once = true;
                }
                "-m" | "--model" => {
                    if let Some(model) = iter.next() {
                        opts.model = match model::Model::from_str(&model) {
                            Ok(model) => model,
                            Err(err) => {
                                println!(
                                    "{} {}",
                                    format!("Could not parse model: {}", err).red(),
                                    "Please enter a valid model.".bright_black()
                                );
                                process::exit(1);
                            }
                        };
                    }
                }
                "-a" | "--auto-commit" => {
                    opts.auto_commmit = true;
                    opts.n = 1;
                    opts.print_once = true;
                }
                "--amend" => {
                    opts.amend = true;
                }
                "--check-version" => {
                    opts.check_version_only = true;
                }
                "--api-endpoint" => {
                    if let Some(endpoint) = iter.next() {
                        opts.api_endpoint = endpoint;
                    }
                }
                "--system-msg-file" => {
                    if let Some(path) = iter.next() {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => opts.system_msg = Some(content),
                            Err(err) => {
                                println!(
                                    "{} {}",
                                    format!("Could not read system message file: {}", err).red(),
                                    "Please provide a valid file path.".bright_black()
                                );
                                process::exit(1);
                            }
                        }
                    }
                }
                "--disable-auto-update-check" => {
                    opts.disable_auto_update_check = true;
                }
                "--api-key" => {
                    if let Some(key) = iter.next() {
                        opts.api_key = Some(key);
                    }
                }
                "-e" | "--reasoning-effort" => {
                    if let Some(effort) = iter.next() {
                        // Support 'none' to disable reasoning, plus low/medium/high
                        if !["none", "low", "medium", "high"].contains(&effort.as_str()) {
                            println!(
                                "{} {}",
                                "Warning: Uncommon reasoning effort value.".yellow(),
                                "Common values are: none, low, medium, high".bright_black()
                            );
                        }
                        opts.reasoning_effort = Some(effort);
                    }
                }
                "-v" | "--verbosity" => {
                    if let Some(level) = iter.next() {
                        if !["low", "medium", "high"].contains(&level.as_str()) {
                            println!(
                                "{} {}",
                                "Warning: Invalid verbosity level.".yellow(),
                                "Valid values are: low, medium, high".bright_black()
                            );
                        }
                        opts.verbosity = Some(level);
                    }
                }
                "-d" | "--debug" => {
                    opts.debug = true;
                    opts.print_once = true;
                }
                "--debug-file" => {
                    if let Some(path) = iter.next() {
                        opts.debug_file = Some(path);
                        opts.debug = true;
                        opts.print_once = true;
                    }
                }
                "--debug-context" => {
                    opts.debug_context = true;
                }
                "--select-files" => {
                    opts.always_select_files = true;
                }
                "-c" | "--config" => {
                    if let Some(path) = iter.next() {
                        opts.config_file = Some(path);
                    }
                }
                "-r" | "--revision" => {
                    if let Some(rev) = iter.next() {
                        opts.jj_revision = Some(rev);
                    }
                }
                "--rw" => {
                    opts.jj_rewrite = !opts.jj_rewrite;
                }
                "-h" | "--help" => help(),
                "--version" => {
                    println!("turbocommit version {}", env!("CARGO_PKG_VERSION").purple());
                    process::exit(0);
                }
                _ => {
                    if arg.starts_with('-') {
                        println!(
                            "{} {} {}",
                            "Unknown option: ".red(),
                            arg.purple().bold(),
                            "\nPlease use -h or --help for help.".bright_black()
                        );
                        process::exit(1);
                    }
                    msg.push_str(&arg);
                    msg.push(' ');
                }
            }
        }
        if !msg.is_empty() {
            opts.msg = format!("User Explanation/Instruction: '{}'", msg.trim());
        }
        opts
    }
}

fn help() {
    println!("{}", "    __             __".red());
    println!("{}", "   / /___  _______/ /_  ____".red());
    println!("{}", "  / __/ / / / ___/ __ \\/ __ \\".yellow());
    println!("{}", " / /_/ /_/ / /  / /_/ / /_/ /".green());
    println!(
        "{}{}",
        " \\__/\\__,_/_/  /_.___/\\____/       ".blue(),
        "_ __".purple()
    );
    println!("{}", "   _________  ____ ___  ____ ___  (_) /_".purple());
    println!("{}", "  / ___/ __ \\/ __ `__ \\/ __ `__ \\/ / __/".red());
    println!("{}", " / /__/ /_/ / / / / / / / / / / / / /_".yellow());
    println!("{}", " \\___/\\____/_/ /_/ /_/_/ /_/ /_/_/\\__/".green());

    println!("\nUsage: turbocommit [options] [message]\n");
    println!("{}", "NOTE: turboCommit now exclusively uses GPT-5.1 models".yellow().bold());
    println!("{}\n", "Only gpt-5.1, gpt-5.1-codex, and gpt-5.1-codex-mini are supported".bright_black());
    println!("Options:");
    println!("  -n <n>   Number of choices to generate (default: 3)\n");
    println!("  -m <m>   Model to use (must be a GPT-5.1 model)\n  --model <m>");
    println!("           Examples: gpt-5.1, gpt-5.1-codex, gpt-5.1-codex-mini\n");
    println!("  -p       Will not print tokens as they are generated.\n  --print-once \n");
    println!("  -a, --auto-commit  Automatically generate and commit a single message\n");
    println!("  --amend  Amend the last commit with the generated message\n");
    println!("  --check-version  Check for updates and exit\n");
    println!("  --api-endpoint <url>  Set the API endpoint URL\n");
    println!("  --system-msg-file <path>  Load system message from a file\n");
    println!("  --disable-auto-update-check  Disable automatic update checks\n");
    println!("  --api-key <key>  Set the API key\n");
    println!("  -e, --reasoning-effort <effort>  Set the reasoning effort level\n");
    println!("                              Values: none, low (default), medium, high\n");
    println!("                              Use 'none' to disable reasoning features\n");
    println!("  -v, --verbosity <level>  Set output verbosity level (default: medium)\n");
    println!("                      Values: low, medium, high\n");
    println!("  -d, --debug  Enable debug mode (shows request/response info and token usage)\n");
    println!("  --debug-file <path>  Write detailed debug logs to specified file (overwrites existing file)\n");
    println!("                       Use '-' to write to stdout instead of a file\n");
    println!("  --debug-context  Log all message contents being sent to the AI\n");
    println!("  --select-files  Always prompt for file selection, regardless of token count\n");
    println!("  -c, --config <path>  Set the config file path\n");
    println!("  -r, --revision <rev>  Set the Jujutsu revision to describe (default: current working directory)\n");
    println!("  --rw  Toggle rewrite mode (inverts config default)\n");
    println!("Anything else will be concatenated into an extra message given to the AI\n");
    println!("You can change the defaults for these options and the system message prompt in the config file, that is created the first time running the program\n{}",
        home::home_dir().unwrap_or_else(|| "".into()).join(".turbocommit.yaml").display());
    println!("To go back to the default system message, delete the config file.\n");
    println!(
        "\nThe system message is about ~{} tokens long",
        format!(
            "{}",
            count_token(&crate::config::Config::load().unwrap_or_else(|e| {
                println!("{}", format!("Error loading config: {}", e).red());
                process::exit(1);
            }).system_msg).unwrap_or(0)
        )
        .green()
    );
    process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_options_from_config() {
        let config = Config::default();
        let options = Options::from(&config);

        assert_eq!(options.n, config.default_number_of_choices);
        assert_eq!(options.print_once, config.disable_print_as_stream);
        assert_eq!(options.model, config.model);
        assert_eq!(options.reasoning_effort, Some(config.reasoning_effort));
        assert_eq!(options.verbosity, Some(config.verbosity));
    }

    #[test]
    fn test_options_new() {
        let config = Config::default();
        let args = vec![
            "turbocommit",
            "-n",
            "3",
            "--print-once",
            "--model",
            "gpt-5.1",
            "--reasoning-effort",
            "medium",
            "--verbosity",
            "high",
            "test",
            "commit",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert_eq!(options.n, 3);
        assert_eq!(options.print_once, true);
        assert_eq!(options.model.0, "gpt-5.1");
        assert_eq!(options.reasoning_effort, Some("medium".to_string()));
        assert_eq!(options.verbosity, Some("high".to_string()));
        assert_eq!(options.msg, "User Explanation/Instruction: 'test commit'");
    }

    #[test]
    fn test_uncommon_reasoning_effort() {
        let config = Config::default();
        let args = vec![
            "turbocommit",
            "--reasoning-effort",
            "very-high",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert_eq!(options.reasoning_effort, Some("very-high".to_string()));
    }

    #[test]
    fn test_debug_mode() {
        let config = Config::default();
        let args = vec![
            "turbocommit",
            "-d",
            "--model",
            "gpt-5.1-codex-mini",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert!(options.debug);
        assert!(options.print_once); // Debug mode forces print_once
        assert_eq!(options.model.0, "gpt-5.1-codex-mini");
    }

    #[test]
    fn test_debug_file_options() {
        let config = Config::default();
        
        // Test debug file to a path
        let args = vec![
            "turbocommit",
            "--debug-file",
            "debug.log",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert!(options.debug);  // Debug mode should be enabled
        assert!(options.print_once);  // Should force print_once
        assert_eq!(options.debug_file, Some("debug.log".to_string()));

        // Test debug file to stdout with "-"
        let args = vec![
            "turbocommit",
            "--debug-file",
            "-",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert!(options.debug);
        assert!(options.print_once);
        assert_eq!(options.debug_file, Some("-".to_string()));

        // Test debug mode without file
        let args = vec![
            "turbocommit",
            "-d",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert!(options.debug);
        assert!(options.print_once);
        assert_eq!(options.debug_file, None);
    }

    #[test]
    fn test_reasoning_none_mode() {
        let config = Config::default();
        let args = vec![
            "turbocommit",
            "--reasoning-effort",
            "none",
            "--model",
            "gpt-5.1",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert_eq!(options.reasoning_effort, Some("none".to_string()));
        assert_eq!(options.model.0, "gpt-5.1");
    }

    #[test]
    fn test_verbosity_option() {
        let config = Config::default();
        
        // Test low verbosity
        let args = vec![
            "turbocommit",
            "--verbosity",
            "low",
            "--model",
            "gpt-5.1-codex-mini",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);
        assert_eq!(options.verbosity, Some("low".to_string()));

        // Test high verbosity
        let args = vec![
            "turbocommit",
            "--verbosity",
            "high",
            "--model",
            "gpt-5.1-codex",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);
        assert_eq!(options.verbosity, Some("high".to_string()));
    }

    #[test]
    fn test_short_options_for_reasoning_and_verbosity() {
        let config = Config::default();
        let args = vec![
            "turbocommit",
            "-e",
            "high",
            "-v",
            "low",
            "--model",
            "gpt-5.1",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert_eq!(options.reasoning_effort, Some("high".to_string()));
        assert_eq!(options.verbosity, Some("low".to_string()));
        assert_eq!(options.model.0, "gpt-5.1");
    }

    #[test]
    fn test_invalid_model_rejected() {
        let _config = Config::default();
        // This should fail during parsing when model validation is enforced
        // The test expects the program to exit with an error
        // Note: This test would need to be run in a subprocess to catch the exit
    }
}
