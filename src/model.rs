use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model(pub String);

impl FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Only allow GPT-5.1 specific models
        const ALLOWED_MODELS: &[&str] = &["gpt-5.1", "gpt-5.1-codex", "gpt-5.1-codex-mini"];
        
        if !ALLOWED_MODELS.contains(&s) {
            return Err(format!(
                "Invalid model '{}'. Only GPT-5.1 models are supported: gpt-5.1, gpt-5.1-codex, gpt-5.1-codex-mini",
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
        // GPT-5.1 specific models context sizes
        match self.0.as_str() {
            "gpt-5.1" => 200000,
            "gpt-5.1-codex" => 200000,
            "gpt-5.1-codex-mini" => 128000,
            _ => 200000, // Default for GPT-5.1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpt51_models_context_sizes() {
        assert_eq!(Model("gpt-5.1".to_string()).context_size(), 200000);
        assert_eq!(Model("gpt-5.1-codex".to_string()).context_size(), 200000);
        assert_eq!(Model("gpt-5.1-codex-mini".to_string()).context_size(), 128000);
    }

    #[test]
    fn test_only_gpt51_models_allowed() {
        // Only GPT-5.1 specific models should be accepted
        assert!(Model::from_str("gpt-5.1").is_ok());
        assert!(Model::from_str("gpt-5.1-codex").is_ok());
        assert!(Model::from_str("gpt-5.1-codex-mini").is_ok());
        
        // All other models should be rejected (including generic gpt-5.x)
        assert!(Model::from_str("gpt-5").is_err());
        assert!(Model::from_str("gpt-5-nano").is_err());
        assert!(Model::from_str("gpt-5-mini").is_err());
        assert!(Model::from_str("gpt-5-codex").is_err());
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
        assert!(err.contains("5.1"));
        
        // Test that even gpt-5 variants are rejected
        let err = Model::from_str("gpt-5").unwrap_err();
        assert!(err.contains("gpt-5"));
        assert!(err.contains("5.1"));
    }
}
