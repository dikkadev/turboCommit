use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum Model {
    Gpt4,
    Gpt4turbo,
    Gpt4o,
    #[default]
    Gpt4omini,
}

impl FromStr for Model {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gpt-4" => Ok(Self::Gpt4),
            "gpt-4-turbo" => Ok(Self::Gpt4turbo),
            "gpt-4o" => Ok(Self::Gpt4o),
            "gpt-4o-mini" => Ok(Self::Gpt4omini),
            _ => Err(format!("{} is not a valid model", s)),
        }
    }
}

impl ToString for Model {
    fn to_string(&self) -> String {
        match self {
            Self::Gpt4 { .. } => String::from("gpt-4"),
            Self::Gpt4turbo { .. } => String::from("gpt-4-turbo"),
            Self::Gpt4o { .. } => String::from("gpt-4o"),
            Self::Gpt4omini { .. } => String::from("gpt-4o-mini"),
        }
    }
}

impl Serialize for Model {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Model {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl Model {
    pub fn all() -> Vec<Self> {
        vec![Self::Gpt4, Self::Gpt4turbo, Self::Gpt4o, Self::Gpt4omini]
    }

    pub fn cost(&self, prompt_tokens: usize, completion_tokens: usize) -> f64 {
        let (prompt_cost, completion_cost) = match self {
            Self::Gpt4 => (30.0, 60.0),
            Self::Gpt4turbo => (10.0, 30.0),
            Self::Gpt4o => (5.0, 15.0),
            Self::Gpt4omini => (0.15, 0.6),
        };
        (prompt_tokens as f64).mul_add(
            prompt_cost / 1000000.0,
            (completion_tokens as f64) * (completion_cost / 1000000.0),
        )
    }

    pub const fn context_size(&self) -> usize {
        match self {
            Self::Gpt4 => 8192,
            Self::Gpt4turbo => 128000,
            Self::Gpt4o => 128000,
            Self::Gpt4omini => 128000,
        }
    }
}
