use crate::model;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::process;
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
pub struct Config {
    #[serde(default)]
    pub model: model::Model,
    #[serde(default)]
    pub api_endpoint: String,
    #[serde(default)]
    pub api_key_env_var: String,
    #[serde(default)]
    pub default_temperature: f64,
    #[serde(default)]
    pub default_frequency_penalty: f64,
    #[serde(default)]
    pub default_number_of_choices: i32,
    #[serde(default)]
    pub disable_print_as_stream: bool,
    #[serde(default)]
    pub disable_auto_update_check: bool,
    #[serde(default)]
    pub enable_reasoning: bool,
    #[serde(default)]
    pub jj_rewrite_default: bool,
    #[serde(default)]
    pub system_msg: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: model::Model("gpt-4o-mini".to_string()),
            api_endpoint: String::from("https://api.openai.com/v1/chat/completions"),
            api_key_env_var: String::from("OPENAI_API_KEY"),
            default_temperature: 1.05,
            default_frequency_penalty: 0.0,
            default_number_of_choices: 3,
            disable_print_as_stream: false,
            disable_auto_update_check: false,
            enable_reasoning: false,
            jj_rewrite_default: false, // Default to overwrite mode
            system_msg: String::from("You are a specialized AI that generates conventional commit messages based on git diffs. Your ONLY purpose is to produce properly formatted conventional commits that follow the exact specification at conventionalcommits.org.

# INPUT AND RESPONSE FORMAT
- You will receive a git diff of staged files
- You MAY additionally receive a single line that begins with \"Description: \" followed by a prior/assistant-provided commit message hint
- You MUST respond ONLY with a single, properly formatted conventional commit message
- Your response must NOT be formatted as markdown or contain any other markup
- Your response must consist of a single headline and optionally one body paragraph
- Never include multiple commits or bullet points in your response

# COMMIT PHILOSOPHY
- Focus primarily on WHY the change was made, not WHAT was changed (the diff already shows the what)
- A good commit explains the intent, motivation, and reasoning behind the change
- Commits should provide context that isn't obvious from the code itself
- Think at a higher abstraction level than the code - capture the purpose, not the implementation

# CONVENTIONAL COMMIT STRUCTURE
<type>[optional scope][!]: <description>

[optional body]

[optional footer(s)]

# COMMIT RULES
1. Type: MUST be one of these nouns:
   - 'feat': introduces a new feature (correlates with MINOR in SemVer)
   - 'fix': patches a bug (correlates with PATCH in SemVer)
   - 'docs': documentation changes only
   - 'style': changes that don't affect code meaning (whitespace, formatting, etc.)
   - 'refac': code change that neither fixes a bug nor adds a feature
   - 'test': adding or correcting tests
   - 'build': changes affecting build system or external dependencies
   - 'ci': changes to CI configuration files and scripts
   - 'chore': other changes

2. Scope: OPTIONAL (but preferred), must be a noun in parentheses describing a section of the codebase
   Example: feat(parser): add ability to parse arrays

3. Breaking Change: Indicated by adding '!' before the colon or by adding a 'BREAKING CHANGE:' footer
   Example: feat(api)!: remove deprecated endpoints

4. Description: MUST immediately follow the colon and space after type/scope
   - Use imperative, present tense: 'add' not 'added' or 'adds'
   - Don't capitalize first letter
   - No period at the end
   - Focus on the intent rather than implementation details
   - Be specific yet concise about the change's purpose

5. Body: OPTIONAL but when present MUST:
   - Be separated from description by a blank line
   - Be a single concise paragraph explaining the motivation and context
   - Focus on WHY the change was needed, not what was changed
   - Explain the problem being solved, not how you solved it
   - Describe intent, rationale, and underlying reasons for the change
   - Highlight non-obvious implications or connections to other parts of the system
   - Never be a list of changes (the git diff already shows this)
   - Follow the KISS principle: brief but meaningful
   - Provide context without being verbose

6. Footer: OPTIONAL, must be separated from body by blank line
   Example: BREAKING CHANGE: configuration format has changed

# EXAMPLES
feat: add user authentication feature
fix(database): resolve connection timeout issue
refactor!: change API response format
chore: update dependencies to latest versions

# HIGH-LEVEL COMMIT EXAMPLES WITH BODY
feat(auth): implement OAuth2 login flow

Enable users to authenticate via third-party providers instead of managing credentials locally, improving security and reducing friction in the sign-up process.

fix(performance): optimize database query pagination

Resolves timeout issues during high traffic periods by implementing cursor-based pagination instead of offset-based, dramatically reducing query execution time.

# ADDITIONAL INSTRUCTIONS
- If a \"Description: \" hint is provided, you MUST actively leverage it: analyze and extract its core intent, motivation, and messaging direction
- The hint reflects the user's understanding of WHY the change matters; use it to strongly shape your type/scope/body choices and to emphasize key reasoning
- Always reword the hint to follow conventional commit style (imperative mood, lowercase start, no period), but preserve and prioritize its semantic content
- Only contradict hint content if it factually conflicts with the diff; otherwise trust and fully incorporate the hint's framing and emphasis
- User may provide specific instructions or additional context - incorporate only if relevant
- User may ask for revisions - be responsive to feedback
- NEVER include explanations about your reasoning or analysis - ONLY output the commit message

Remember: Always prioritize clarity and precision over verbosity."),
        }
    }
}

impl Config {
    pub fn load_from_path(path: &std::path::Path) -> anyhow::Result<Self> {
        //debug log the path we load from
        println!("Loading config from path: {}", path.display());
        let config = match std::fs::read_to_string(path) {
            Ok(config_str) => {
                match serde_yaml::from_str::<Self>(&config_str) {
                    Ok(config) => config,
                    Err(err) => {
                        return Err(anyhow::anyhow!("Configuration file parsing error: {}", err));
                    }
                }
            },
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::NotFound => {
                        println!("{}", format!("Config file not found at: {}", path.display()).red());
                        process::exit(1);
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Error reading configuration file: {}", err));
                    }
                }
            }
        };

        // Validate the configuration
        if let Err(validation_errors) = config.validate() {
            let mut error_msg = String::from("Configuration validation errors:\n");
            for error in validation_errors {
                error_msg.push_str(&format!("  {}\n", error));
            }
            error_msg.push_str(&format!("\nConfiguration file location: {}", path.display()));
            
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

        let config = match std::fs::read_to_string(&path) {
            Ok(config_str) => {
                match serde_yaml::from_str::<Self>(&config_str) {
                    Ok(config) => config,
                    Err(err) => {
                        return Err(anyhow::anyhow!("Configuration file parsing error: {}", err));
                    }
                }
            },
            Err(err) => {
                match err.kind() {
                    std::io::ErrorKind::NotFound => {
                        println!("{}", "No configuration file found, creating one with default values.".bright_black());
                        let default = Self::default();
                        if let Err(e) = default.save_if_changed() {
                            println!("{}", format!("Warning: Failed to create default config file: {}", e).yellow());
                        }
                        default
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Error reading configuration file: {}", err));
                    }
                }
            }
        };

        // Validate the configuration
        if let Err(validation_errors) = config.validate() {
            let mut error_msg = String::from("Configuration validation errors:\n");
            for error in validation_errors {
                error_msg.push_str(&format!("  {}\n", error));
            }
            error_msg.push_str(&format!("\nConfiguration file location: {}", path.display()));
            
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

    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        let default = Self::default();

        // Validate model
        if self.model.0.is_empty() {
            errors.push(ValidationError {
                field: "model".to_string(),
                message: format!("Model cannot be empty (default: {})", default.model.0),
            });
        }

        // Validate API endpoint
        if let Err(_) = Url::parse(&self.api_endpoint) {
            errors.push(ValidationError {
                field: "api_endpoint".to_string(),
                message: format!("Invalid URL format (default: {})", default.api_endpoint),
            });
        }

        // Validate temperature
        if !(0.0..=2.0).contains(&self.default_temperature) {
            errors.push(ValidationError {
                field: "default_temperature".to_string(),
                message: format!("Temperature must be between 0.0 and 2.0 (default: {})", default.default_temperature),
            });
        }

        // Validate frequency penalty
        if !(-2.0..=2.0).contains(&self.default_frequency_penalty) {
            errors.push(ValidationError {
                field: "default_frequency_penalty".to_string(),
                message: format!("Frequency penalty must be between -2.0 and 2.0 (default: {})", default.default_frequency_penalty),
            });
        }

        // Validate number of choices
        if self.default_number_of_choices < 1 {
            errors.push(ValidationError {
                field: "default_number_of_choices".to_string(),
                message: format!("Number of choices must be at least 1 (default: {})", default.default_number_of_choices),
            });
        }

        // Validate system message
        if self.system_msg.trim().is_empty() {
            errors.push(ValidationError {
                field: "system_msg".to_string(),
                message: "System message cannot be empty (see default message below)".to_string(),
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

    #[test]
    fn test_default_config_is_valid() {
        let config = Config::default();
        assert!(config.validate().is_ok());
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
    fn test_validate_invalid_temperature() {
        let mut config = Config::default();
        config.default_temperature = 2.5;
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "default_temperature");
    }

    #[test]
    fn test_validate_invalid_frequency_penalty() {
        let mut config = Config::default();
        config.default_frequency_penalty = -3.0;
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "default_frequency_penalty");
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
        config.default_temperature = 3.0;
        config.system_msg = "".to_string();
        let errors = config.validate().unwrap_err();
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn test_load_valid_config() {
        let config_content = r#"
model: gpt-4
api_endpoint: https://api.openai.com/v1/chat/completions
default_temperature: 1.0
default_frequency_penalty: 0.0
default_number_of_choices: 3
disable_print_as_stream: false
disable_auto_update_check: true
system_msg: "Test message"
"#;
        let (_file_path, _dir) = create_test_config(config_content);
        
        // Set the home directory to our temp directory for this test
        std::env::set_var("HOME", _dir.path());
        
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
        std::env::set_var("HOME", _dir.path());
        
        let config = Config::load();
        assert!(config.is_err(), "Expected config loading to fail with invalid YAML");
    }

    #[test]
    fn test_load_missing_file_creates_default() {
        let _dir = tempdir().unwrap();
        std::env::set_var("HOME", _dir.path());
        
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
        config.default_temperature = 3.0;
        
        let errors = config.validate().unwrap_err();
        let default = Config::default();
        
        // Find the model error
        let model_error = errors.iter().find(|e| e.field == "model").unwrap();
        assert!(model_error.message.contains(&default.model.0));
        
        // Find the temperature error
        let temp_error = errors.iter().find(|e| e.field == "default_temperature").unwrap();
        assert!(temp_error.message.contains(&default.default_temperature.to_string()));
    }

    #[test]
    fn test_empty_system_msg_shows_default() {
        let config_content = r#"
model: gpt-4
api_endpoint: https://api.openai.com/v1/chat/completions
default_temperature: 1.0
default_frequency_penalty: 0.0
default_number_of_choices: 3
disable_print_as_stream: false
disable_auto_update_check: false
system_msg: ""
"#;
        let (_file_path, _dir) = create_test_config(config_content);
        std::env::set_var("HOME", _dir.path());
        
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
        std::env::set_var("HOME", _dir.path());
        
        // Create a config with some changes
        let mut config = Config::default();
        config.model = model::Model("gpt-4".to_string());
        
        // First save should succeed
        assert!(config.save_if_changed().is_ok());
        
        // Second save with no changes should still be ok
        assert!(config.save_if_changed().is_ok());
        
        // Verify the file was created with correct content
        let config_path = _dir.path().join(".turbocommit.yaml");
        assert!(config_path.exists());
        let content = std::fs::read_to_string(config_path).unwrap();
        let loaded_config: Config = serde_yaml::from_str(&content).unwrap();
        assert_eq!(loaded_config.model.0, "gpt-4");
    }

    #[test]
    fn test_default_auto_update_check() {
        let config = Config::default();
        assert!(!config.disable_auto_update_check, "Auto update check should be enabled by default");
    }

    #[test]
    fn test_load_from_path_valid_config() {
        let config_content = r#"
model: gpt-4
api_endpoint: https://api.openai.com/v1/chat/completions
default_temperature: 1.0
default_frequency_penalty: 0.0
default_number_of_choices: 3
disable_print_as_stream: false
disable_auto_update_check: true
system_msg: "Test message"
"#;
        let (file_path, _dir) = create_test_config(config_content);
        
        let config = Config::load_from_path(&file_path);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.model.0, "gpt-4");
        assert_eq!(config.default_temperature, 1.0);
        assert!(config.disable_auto_update_check);
        assert_eq!(config.system_msg, "Test message");
    }

    #[test]
    fn test_load_from_path_invalid_yaml() {
        let config_content = "invalid: yaml: content: [";
        let (file_path, _dir) = create_test_config(config_content);
        
        let config = Config::load_from_path(&file_path);
        assert!(config.is_err());
        assert!(config.unwrap_err().to_string().contains("Configuration file parsing error"));
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
default_temperature: 3.0  # Out of range
default_frequency_penalty: 0.0
default_number_of_choices: 3
disable_print_as_stream: false
disable_auto_update_check: false
system_msg: "Test message"
"#;
        let (file_path, _dir) = create_test_config(config_content);
        
        let config = Config::load_from_path(&file_path);
        assert!(config.is_err());
        let err = config.unwrap_err().to_string();
        assert!(err.contains("model"));  // Should mention empty model error
        assert!(err.contains("api_endpoint"));  // Should mention invalid URL error
        assert!(err.contains("temperature"));  // Should mention temperature range error
    }

    #[test]
    fn test_load_from_path_empty_system_msg() {
        let config_content = r#"
model: "gpt-4"
api_endpoint: "https://api.openai.com/v1/chat/completions"
default_temperature: 1.0
default_frequency_penalty: 0.0
default_number_of_choices: 3
disable_print_as_stream: false
disable_auto_update_check: false
system_msg: ""
"#;
        let (file_path, _dir) = create_test_config(config_content);
        
        let config = Config::load_from_path(&file_path);
        assert!(config.is_err());
        let err = config.unwrap_err().to_string();
        assert!(err.contains("system_msg"));  // Should mention system message error
        assert!(err.contains("Default system message:"));  // Should show default message
    }
}
