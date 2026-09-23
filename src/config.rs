use crate::model;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{process, str::FromStr};
use url::Url;

#[derive(Debug)]
pub struct ValidationError {
    field: String,
    message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field.red(), self.message)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub model: model::Model,
    pub api_endpoint: String,
    pub api_key_env_var: String,
    pub default_number_of_choices: i32,
    pub disable_auto_update_check: bool,
    pub reasoning_effort: String,
    pub verbosity: String,
    pub jj_rewrite_default: bool,
    #[serde(skip_serializing_if = "is_default_system_msg")]
    pub system_msg: String,
}

fn is_default_system_msg(value: &str) -> bool {
    value == include_str!("default_prompt.txt")
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: model::Model("gpt-6-luna".to_string()),
            api_endpoint: String::from("https://api.openai.com/v1/chat/completions"),
            api_key_env_var: String::from("OPENAI_API_KEY"),
            default_number_of_choices: 1,
            disable_auto_update_check: false,
            reasoning_effort: String::from("low"),
            verbosity: String::from("medium"),
            jj_rewrite_default: false, // Default to overwrite mode
            system_msg: String::from(include_str!("default_prompt.txt")),
        }
    }
}

impl Config {
    pub fn load_from_path(path: &std::path::Path) -> anyhow::Result<Self> {
        //debug log the path we load from
        println!("Loading config from path: {}", path.display());
        let mut config = match std::fs::read_to_string(path) {
            Ok(config_str) => match serde_yaml::from_str::<Self>(&config_str) {
                Ok(config) => config,
                Err(err) => {
                    return Err(anyhow::anyhow!("Configuration file parsing error: {}", err));
                }
            },
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => {
                    let msg = format!("Config file not found at: {}", path.display());
                    return Err(anyhow::anyhow!(msg));
                }
                _ => {
                    return Err(anyhow::anyhow!("Error reading configuration file: {}", err));
                }
            },
        };

        config.upgrade_legacy_defaults();

        // Validate the configuration
        if let Err(validation_errors) = config.validate() {
            let mut error_msg = String::from("Configuration validation errors:\n");
            for error in validation_errors {
                error_msg.push_str(&format!("  {}\n", error));
            }
            error_msg.push_str(&format!(
                "\nConfiguration file location: {}",
                path.display()
            ));

            // If system message is empty, show the default
            if config.system_msg.trim().is_empty() {
                error_msg.push_str("\n\nDefault system message:\n");
                error_msg.push_str(&Self::default().system_msg);
            }

            return Err(anyhow::anyhow!(error_msg));
        }

        // After validation passes, fill in empty system message with default
        let mut config = config;
        if config.system_msg.trim().is_empty() {
            config.system_msg = Self::default().system_msg;
        }

        Ok(config)
    }

    pub fn load() -> anyhow::Result<Self> {
        let path = home::home_dir().map_or_else(
            || {
                println!("{}", "Unable to find home directory.".red());
                process::exit(1);
            },
            |path| path.join(".turbocommit.yaml"),
        );

        let mut config = match std::fs::read_to_string(&path) {
            Ok(config_str) => match serde_yaml::from_str::<Self>(&config_str) {
                Ok(config) => config,
                Err(err) => {
                    return Err(anyhow::anyhow!("Configuration file parsing error: {}", err));
                }
            },
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => {
                    println!(
                        "{}",
                        "No configuration file found, creating one with default values."
                            .bright_black()
                    );
                    let default = Self::default();
                    if let Err(e) = default.save_if_changed() {
                        println!(
                            "{}",
                            format!("Warning: Failed to create default config file: {}", e)
                                .yellow()
                        );
                    }
                    default
                }
                _ => {
                    return Err(anyhow::anyhow!("Error reading configuration file: {}", err));
                }
            },
        };

        config.upgrade_legacy_defaults();

        // Validate the configuration
        if let Err(validation_errors) = config.validate() {
            let mut error_msg = String::from("Configuration validation errors:\n");
            for error in validation_errors {
                error_msg.push_str(&format!("  {}\n", error));
            }
            error_msg.push_str(&format!(
                "\nConfiguration file location: {}",
                path.display()
            ));

            // If system message is empty, show the default
            if config.system_msg.trim().is_empty() {
                error_msg.push_str("\n\nDefault system message:\n");
                error_msg.push_str(&Self::default().system_msg);
            }

            return Err(anyhow::anyhow!(error_msg));
        }

        // After validation passes, fill in empty system message with default
        let mut config = config;
        if config.system_msg.trim().is_empty() {
            config.system_msg = Self::default().system_msg;
        }

        Ok(config)
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

    fn upgrade_legacy_defaults(&mut self) {
        // Config files generated by 3.x contain the old model and full default prompt.
        // Keep user-written prompts and nondefault suggestion counts intact.
        let legacy_prompt = include_str!("legacy_gpt54_prompt.txt");
        let has_legacy_prompt =
            self.system_msg.replace("\r\n", "\n") == legacy_prompt.replace("\r\n", "\n");
        if self.model.0 == "gpt-5.4" {
            self.model = model::Model("gpt-6-luna".to_string());
            if has_legacy_prompt && self.default_number_of_choices == 3 {
                self.default_number_of_choices = 1;
            }
        }
        if has_legacy_prompt {
            self.system_msg = Self::default().system_msg;
        }
    }

    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        let default = Self::default();

        // Validate model
        if self.model.0.is_empty() {
            errors.push(ValidationError {
                field: "model".to_string(),
                message: format!("Model cannot be empty (default: {})", default.model.0),
            });
        } else if let Err(err) = model::Model::from_str(&self.model.0) {
            errors.push(ValidationError {
                field: "model".to_string(),
                message: err,
            });
        }

        // Validate API endpoint
        if let Err(_) = Url::parse(&self.api_endpoint) {
            errors.push(ValidationError {
                field: "api_endpoint".to_string(),
                message: format!("Invalid URL format (default: {})", default.api_endpoint),
            });
        }

        // Validate number of choices
        if self.default_number_of_choices < 1 {
            errors.push(ValidationError {
                field: "default_number_of_choices".to_string(),
                message: format!(
                    "Number of choices must be at least 1 (default: {})",
                    default.default_number_of_choices
                ),
            });
        }

        // Validate system message
        if self.system_msg.trim().is_empty() {
            errors.push(ValidationError {
                field: "system_msg".to_string(),
                message: "System message cannot be empty (see default message below)".to_string(),
            });
        }

        if !["none", "low", "medium", "high", "xhigh", "max"]
            .contains(&self.reasoning_effort.as_str())
        {
            errors.push(ValidationError {
                field: "reasoning_effort".to_string(),
                message: "Use none, low, medium, high, xhigh, or max".to_string(),
            });
        } else if self.model.0 == "gpt-6-astra" && self.reasoning_effort == "none" {
            errors.push(ValidationError {
                field: "reasoning_effort".to_string(),
                message: "gpt-6-astra requires low or higher reasoning effort".to_string(),
            });
        }

        if !["low", "medium", "high"].contains(&self.verbosity.as_str()) {
            errors.push(ValidationError {
                field: "verbosity".to_string(),
                message: "Use low, medium, or high".to_string(),
            });
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn create_test_config(content: &str) -> (std::path::PathBuf, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(".turbocommit.yaml");
        fs::write(&file_path, content).unwrap();
        (file_path, dir)
    }

    fn set_test_home(path: &std::path::Path) {
        std::env::set_var("HOME", path);
        std::env::set_var("USERPROFILE", path);
    }

    #[test]
    fn test_default_config_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.model.0, "gpt-6-luna");
        assert_eq!(config.default_number_of_choices, 1);
    }

    #[test]
    fn test_partial_config_uses_new_defaults() {
        let (path, _dir) = create_test_config("disable_auto_update_check: true\n");
        let config = Config::load_from_path(&path).unwrap();
        assert_eq!(config.model.0, "gpt-6-luna");
        assert_eq!(config.default_number_of_choices, 1);
        assert!(config.disable_auto_update_check);
        assert_eq!(config.system_msg, Config::default().system_msg);
    }

    #[test]
    fn test_generated_config_omits_default_prompt() {
        let yaml = serde_yaml::to_string(&Config::default()).unwrap();
        assert!(!yaml.contains("system_msg:"));
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.system_msg, Config::default().system_msg);
    }

    #[test]
    fn test_generated_legacy_config_upgrades_in_memory() {
        let mut legacy = Config::default();
        legacy.model = model::Model("gpt-5.4".to_string());
        legacy.default_number_of_choices = 3;
        legacy.system_msg = include_str!("legacy_gpt54_prompt.txt").to_string();
        let (path, _dir) = create_test_config(&serde_yaml::to_string(&legacy).unwrap());
        let config = Config::load_from_path(&path).unwrap();
        assert_eq!(config.model.0, "gpt-6-luna");
        assert_eq!(config.default_number_of_choices, 1);
        assert_eq!(config.system_msg, Config::default().system_msg);
    }

    #[test]
    fn test_legacy_model_keeps_custom_prompt_and_choice_count() {
        let mut legacy = Config::default();
        legacy.model = model::Model("gpt-5.4".to_string());
        legacy.default_number_of_choices = 3;
        legacy.system_msg = "My custom prompt".to_string();
        let (path, _dir) = create_test_config(&serde_yaml::to_string(&legacy).unwrap());
        let config = Config::load_from_path(&path).unwrap();
        assert_eq!(config.model.0, "gpt-6-luna");
        assert_eq!(config.default_number_of_choices, 3);
        assert_eq!(config.system_msg, "My custom prompt");
    }

    #[test]
    fn test_astra_rejects_none_reasoning() {
        let mut config = Config::default();
        config.model = model::Model("gpt-6-astra".to_string());
        config.reasoning_effort = "none".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_empty_model() {
        let mut config = Config::default();
        config.model = model::Model(String::new());
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "model");
    }

    #[test]
    fn test_validate_invalid_api_endpoint() {
        let mut config = Config::default();
        config.api_endpoint = "not a url".to_string();
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "api_endpoint");
    }

    #[test]
    fn test_validate_invalid_number_of_choices() {
        let mut config = Config::default();
        config.default_number_of_choices = 0;
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "default_number_of_choices");
    }

    #[test]
    fn test_validate_empty_system_msg() {
        let mut config = Config::default();
        config.system_msg = "".to_string();
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "system_msg");
    }

    #[test]
    fn test_validate_multiple_errors() {
        let mut config = Config::default();
        config.model = model::Model(String::new());
        config.system_msg = "".to_string();
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
model: gpt-6-luna
api_endpoint: https://api.openai.com/v1/chat/completions
default_number_of_choices: 3
disable_auto_update_check: true
system_msg: "Test message"
"#;
        let (_file_path, _dir) = create_test_config(config_content);

        // Set the home directory to our temp directory for this test
        set_test_home(_dir.path());

        let config = Config::load();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.disable_auto_update_check);
    }

    #[test]
    fn test_load_invalid_yaml() {
        let config_content = "invalid: yaml: content: [";
        let (_file_path, _dir) = create_test_config(config_content);

        // Set the home directory to our temp directory for this test
        set_test_home(_dir.path());

        let config = Config::load();
        assert!(
            config.is_err(),
            "Expected config loading to fail with invalid YAML"
        );
    }

    #[test]
    fn test_load_missing_file_creates_default() {
        let _dir = tempdir().unwrap();
        set_test_home(_dir.path());

        // First load should create the file
        let config = Config::load();
        assert!(config.is_ok());

        // Verify the file was created
        let config_path = _dir.path().join(".turbocommit.yaml");
        assert!(config_path.exists());

        // Verify content matches default
        let content = std::fs::read_to_string(config_path).unwrap();
        let loaded_config: Config = serde_yaml::from_str(&content).unwrap();
        assert_eq!(loaded_config.model.0, Config::default().model.0);
        assert_eq!(loaded_config.api_endpoint, Config::default().api_endpoint);
    }

    #[test]
    fn test_validation_error_includes_defaults() {
        let mut config = Config::default();
        config.model = model::Model(String::new());

        let errors = config.validate().unwrap_err();
        let default = Config::default();

        // Find the model error
        let model_error = errors.iter().find(|e| e.field == "model").unwrap();
        assert!(model_error.message.contains(&default.model.0));
    }

    #[test]
    fn test_empty_system_msg_shows_default() {
        let config_content = r#"
model: gpt-6-luna
api_endpoint: https://api.openai.com/v1/chat/completions
default_number_of_choices: 3
disable_auto_update_check: false
system_msg: ""
"#;
        let (_file_path, _dir) = create_test_config(config_content);
        set_test_home(_dir.path());

        let error = Config::load().unwrap_err();
        let error_msg = error.to_string();

        // Error should contain the default system message
        assert!(error_msg.contains("Default system message:"));
        assert!(error_msg.contains(&Config::default().system_msg));
    }

    #[test]
    fn test_save_if_changed() {
        let _dir = tempdir().unwrap();
        // Set the home directory to our temp directory for this test
        set_test_home(_dir.path());

        // Create a config with some changes
        let mut config = Config::default();
        config.model = model::Model("gpt-6-luna".to_string());

        // First save should succeed
        assert!(config.save_if_changed().is_ok());

        // Second save with no changes should still be ok
        assert!(config.save_if_changed().is_ok());

        // Verify the file was created with correct content
        let config_path = _dir.path().join(".turbocommit.yaml");
        assert!(config_path.exists());
        let content = std::fs::read_to_string(config_path).unwrap();
        let loaded_config: Config = serde_yaml::from_str(&content).unwrap();
        assert_eq!(loaded_config.model.0, "gpt-6-luna");
    }

    #[test]
    fn test_default_auto_update_check() {
        let config = Config::default();
        assert!(
            !config.disable_auto_update_check,
            "Auto update check should be enabled by default"
        );
    }

    #[test]
    fn test_load_from_path_valid_config() {
        let config_content = r#"
model: gpt-6-luna
api_endpoint: https://api.openai.com/v1/chat/completions
default_number_of_choices: 3
disable_auto_update_check: true
system_msg: "Test message"
"#;
        let (file_path, _dir) = create_test_config(config_content);

        let config = Config::load_from_path(&file_path);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.model.0, "gpt-6-luna");
        assert!(config.disable_auto_update_check);
        assert_eq!(config.system_msg, "Test message");
    }

    #[test]
    fn test_load_from_path_invalid_yaml() {
        let config_content = "invalid: yaml: content: [";
        let (file_path, _dir) = create_test_config(config_content);

        let config = Config::load_from_path(&file_path);
        assert!(config.is_err());
        assert!(config
            .unwrap_err()
            .to_string()
            .contains("Configuration file parsing error"));
    }

    #[test]
    fn test_load_from_path_nonexistent_file() {
        let dir = tempdir().unwrap();
        let nonexistent_path = dir.path().join("nonexistent.yaml");

        let config = Config::load_from_path(&nonexistent_path);
        assert!(config.is_err());
    }

    #[test]
    fn test_load_from_path_invalid_config() {
        let config_content = r#"
model: ""  # Empty model is invalid
api_endpoint: not-a-url
default_number_of_choices: 3
disable_auto_update_check: false
system_msg: "Test message"
"#;
        let (file_path, _dir) = create_test_config(config_content);

        let config = Config::load_from_path(&file_path);
        assert!(config.is_err());
        let err = config.unwrap_err().to_string();
        assert!(err.contains("model")); // Should mention empty model error
        assert!(err.contains("api_endpoint")); // Should mention invalid URL error
    }

    #[test]
    fn test_load_from_path_invalid_model() {
        let config_content = r#"
model: "gpt-6-luna-pro"
api_endpoint: "https://api.openai.com/v1/chat/completions"
default_number_of_choices: 3
disable_auto_update_check: false
system_msg: "Test message"
"#;
        let (file_path, _dir) = create_test_config(config_content);

        let config = Config::load_from_path(&file_path);
        assert!(config.is_err());
        let err = config.unwrap_err().to_string();
        assert!(err.contains("Supported models: gpt-6-luna"));
    }

    #[test]
    fn test_load_from_path_empty_system_msg() {
        let config_content = r#"
model: "gpt-6-luna"
api_endpoint: "https://api.openai.com/v1/chat/completions"
default_number_of_choices: 3
disable_auto_update_check: false
system_msg: ""
"#;
        let (file_path, _dir) = create_test_config(config_content);

        let config = Config::load_from_path(&file_path);
        assert!(config.is_err());
        let err = config.unwrap_err().to_string();
        assert!(err.contains("system_msg")); // Should mention system message error
        assert!(err.contains("Default system message:")); // Should show default message
    }
}
