use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Model(pub String);

impl FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
        // Default to a conservative context size if unknown
        match self.0.as_str() {
            "gpt-4" => 8192,
            "gpt-4-turbo" => 128000,
            "gpt-4o" => 128000,
            "gpt-4o-mini" => 128000,
            "o1" => 200000,
            "o1-mini" => 128000,
            "o1-preview" => 128000,
            "o3-mini" => 200000,
            _ => 4096, // Conservative default
        }
    }
}
