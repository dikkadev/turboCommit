use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model(pub String);

impl FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const ALLOWED_MODELS: &[&str] = &["gpt-5.4"];

        if !ALLOWED_MODELS.contains(&s) {
            return Err(format!("Invalid model '{}'. Only gpt-5.4 is supported", s));
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
        match self.0.as_str() {
            "gpt-5.4" => 1_050_000,
            _ => 1_050_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpt54_context_size() {
        assert_eq!(Model("gpt-5.4".to_string()).context_size(), 1_050_000);
    }

    #[test]
    fn test_only_gpt54_allowed() {
        assert!(Model::from_str("gpt-5.4").is_ok());

        assert!(Model::from_str("gpt-5").is_err());
        assert!(Model::from_str("gpt-5.4-pro").is_err());
        assert!(Model::from_str("gpt-5-mini").is_err());
        assert!(Model::from_str("gpt-5-nano").is_err());
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
        assert!(err.contains("gpt-5.4"));

        let err = Model::from_str("gpt-5").unwrap_err();
        assert!(err.contains("gpt-5"));
        assert!(err.contains("gpt-5.4"));
    }
}
