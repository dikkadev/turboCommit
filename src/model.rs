use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model(pub String);

impl FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Only allow GPT-5.x models (GPT-5.1 and variants)
        if !s.starts_with("gpt-5") {
            return Err(format!(
                "Invalid model '{}'. Only GPT-5.x models are supported (e.g., gpt-5, gpt-5-nano, gpt-5-mini, gpt-5-codex)",
                s
            ));
        }
        Ok(Self(s.to_string()))
    }
}

impl ToString for Model {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Serialize for Model {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Model {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s))
    }
}

impl Model {
    pub fn context_size(&self) -> usize {
        // All GPT-5.x models - context sizes based on variant
        match self.0.as_str() {
            "gpt-5-nano" => 128000,
            "gpt-5-mini" => 128000,
            "gpt-5" => 200000,
            "gpt-5-codex" => 128000,
            _ => {
                // Default for any other gpt-5.x variant
                if self.0.starts_with("gpt-5") {
                    200000
                } else {
                    usize::MAX
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpt5_models_context_sizes() {
        assert_eq!(Model("gpt-5-nano".to_string()).context_size(), 128000);
        assert_eq!(Model("gpt-5-mini".to_string()).context_size(), 128000);
        assert_eq!(Model("gpt-5".to_string()).context_size(), 200000);
        assert_eq!(Model("gpt-5-codex".to_string()).context_size(), 128000);
    }

    #[test]
    fn test_gpt5_variants_use_default() {
        // Any gpt-5 variant not explicitly listed should get default context size
        assert_eq!(Model("gpt-5-experimental".to_string()).context_size(), 200000);
        assert_eq!(Model("gpt-5-turbo".to_string()).context_size(), 200000);
    }

    #[test]
    fn test_only_gpt5_models_allowed() {
        // Only gpt-5.x models should be accepted
        assert!(Model::from_str("gpt-5").is_ok());
        assert!(Model::from_str("gpt-5-nano").is_ok());
        assert!(Model::from_str("gpt-5-mini").is_ok());
        
        // Old models should be rejected
        assert!(Model::from_str("gpt-4").is_err());
        assert!(Model::from_str("gpt-4o").is_err());
        assert!(Model::from_str("o1").is_err());
        assert!(Model::from_str("o3-mini").is_err());
        assert!(Model::from_str("unknown-model").is_err());
    }

    #[test]
    fn test_model_validation_error_message() {
        let err = Model::from_str("gpt-4").unwrap_err();
        assert!(err.contains("gpt-4"));
        assert!(err.contains("GPT-5"));
    }
}
