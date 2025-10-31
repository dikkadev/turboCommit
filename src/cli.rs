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
    pub t: f64,
    pub f: f64,
    pub print_once: bool,
    pub model: model::Model,
    pub auto_commmit: bool,
    pub check_version_only: bool,
    pub api_endpoint: String,
    pub system_msg: Option<String>,
    pub disable_auto_update_check: bool,
    pub api_key: Option<String>,
    pub reasoning_effort: Option<String>,
    pub enable_reasoning: bool,
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
            t: config.default_temperature,
            f: config.default_frequency_penalty,
            print_once: config.disable_print_as_stream,
            model: config.model.clone(),
            auto_commmit: false,
            check_version_only: false,
            api_endpoint: config.api_endpoint.clone(),
            system_msg: None,
            disable_auto_update_check: config.disable_auto_update_check,
            api_key: None,
            reasoning_effort: None,
            enable_reasoning: config.enable_reasoning,
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
                "-t" => {
                    if let Some(t) = iter.next() {
                        opts.t = t.parse().map_or_else(
                            |_| {
                                println!(
                                    "{} {}",
                                    "Could not parse t.".red(),
                                    "Please enter a float between 0 and 2.".bright_black()
                                );
                                process::exit(1);
                            },
                            |t: f64| t.clamp(0.0, 2.0),
                        );
                    }
                }
                "-f" => {
                    if let Some(f) = iter.next() {
                        opts.f = f.parse().map_or_else(
                            |_| {
                                println!(
                                    "{} {}",
                                    "Could not parse f.".red(),
                                    "Please enter a float between -2.0 and 2.0.".bright_black()
                                );
                                process::exit(1);
                            },
                            |f: f64| f.clamp(-2.0, 2.0),
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
                "--reasoning-effort" => {
                    if let Some(effort) = iter.next() {
                        if !["low", "medium", "high"].contains(&effort.as_str()) {
                            println!(
                                "{} {}",
                                "Warning: Uncommon reasoning effort value.".yellow(),
                                "Common values are: low, medium, high (depends on model/service)".bright_black()
                            );
                        }
                        opts.reasoning_effort = Some(effort);
                    }
                }
                "--reason" | "--enable-reasoning" => {
                    opts.enable_reasoning = true;
                    if opts.reasoning_effort.is_none() {
                        opts.reasoning_effort = Some("medium".to_string());
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
                "-v" | "--version" => {
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
    println!("Options:");
    println!("  -n <n>   Number of choices to generate");
    println!("           Note: Some models (e.g., o-series) may not support multiple choices\n");
    println!("  -m <m>   Model to use\n  --model <m>",);
    println!("    Model can be any OpenAI compatible model name\n");
    println!("  -p       Will not print tokens as they are generated.\n  --print-once \n",);
    println!(
        "  -t <t>   Temperature (|t| 0.0 < t < 2.0)\n{}\n           Note: Has no effect when using reasoning mode\n",
        "(https://platform.openai.com/docs/api-reference/chat/create#chat/create-temperature)"
            .bright_black()
    );
    println!(
        "  -f <f>   Frequency penalty (|f| -2.0 < f < 2.0)\n{}\n",
        "(https://platform.openai.com/docs/api-reference/chat/create#chat/create-frequency-penalty)"
            .bright_black()
    );
    println!("  -a, --auto-commit  Automatically generate and commit a single message\n");
    println!("  --amend  Amend the last commit with the generated message\n");
    println!("  --check-version  Check for updates and exit\n");
    println!("  --api-endpoint <url>  Set the API endpoint URL\n");
    println!("  --system-msg-file <path>  Load system message from a file\n");
    println!("  --disable-auto-update-check  Disable automatic update checks\n");
    println!("  --api-key <key>  Set the API key\n");
    println!("  --reason, --enable-reasoning  Enable support for models with reasoning capabilities (like o-series)\n");
    println!("  --reasoning-effort <effort>  Set the reasoning effort (defaults to 'medium', common values: low, medium, high)\n");
    println!("                              Note: Valid values depend on the model and service being used\n");
    println!("  -d, --debug  Enable debug mode (prints basic request/response info)\n");
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
        assert_eq!(options.t, config.default_temperature);
        assert_eq!(options.f, config.default_frequency_penalty);
        assert_eq!(options.print_once, config.disable_print_as_stream);
        assert_eq!(options.model, config.model);
        assert_eq!(options.enable_reasoning, config.enable_reasoning);
        assert_eq!(options.reasoning_effort, None);
    }

    #[test]
    fn test_options_new() {
        let config = Config::default();
        let args = vec![
            "turbocommit",
            "-n",
            "3",
            "-t",
            "1.0",
            "-f",
            "0.5",
            "--print-once",
            "--model",
            "gpt-4",
            "--enable-reasoning",
            "--reasoning-effort",
            "medium",
            "test",
            "commit",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert_eq!(options.n, 3);
        assert_eq!(options.t, 1.0);
        assert_eq!(options.f, 0.5);
        assert_eq!(options.print_once, true);
        assert_eq!(options.model.0, "gpt-4");
        assert_eq!(options.enable_reasoning, true);
        assert_eq!(options.reasoning_effort, Some("medium".to_string()));
        assert_eq!(options.msg, "User Explanation/Instruction: 'test commit'");
    }

    #[test]
    fn test_uncommon_reasoning_effort() {
        let config = Config::default();
        let args = vec![
            "turbocommit",
            "--enable-reasoning",
            "--reasoning-effort",
            "very-high",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert_eq!(options.enable_reasoning, true);
        assert_eq!(options.reasoning_effort, Some("very-high".to_string()));
    }

    #[test]
    fn test_debug_mode() {
        let config = Config::default();
        let args = vec![
            "turbocommit",
            "-d",
            "--reason",
            "--model",
            "o3-mini",
        ];
        let args = args.into_iter().map(String::from).collect::<Vec<String>>();
        let options = Options::new(args.into_iter(), &config);

        assert!(options.debug);
        assert!(options.print_once); // Debug mode forces print_once
        assert!(options.enable_reasoning);
        assert_eq!(options.reasoning_effort, Some("medium".to_string())); // Default effort
        assert_eq!(options.model.0, "o3-mini");
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
}
