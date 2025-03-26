use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeedingMode {
    Standard,
    Accelerated,
}

impl FromStr for SeedingMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Standard" => Ok(SeedingMode::Standard),
            "Accelerated" => Ok(SeedingMode::Accelerated),
            _ => Err(format!("Unknown seeding mode: {}", s)),
        }
    }
}

impl ToString for SeedingMode {
    fn to_string(&self) -> String {
        match self {
            SeedingMode::Standard => "Standard".to_string(),
            SeedingMode::Accelerated => "Accelerated".to_string(),
        }
    }
}

impl Default for SeedingMode {
    fn default() -> Self {
        Self::Standard
    }
}
