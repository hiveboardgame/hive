use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub enum GameTypeFilter {
    Base,
    Full,
    #[default]
    All,
}

impl GameTypeFilter {
    pub fn all() -> Vec<Self> {
        vec![Self::Full, Self::Base, Self::All]
    }

    pub fn to_db_filter(&self) -> Vec<&'static str> {
        match self {
            Self::Full => vec!["Base+MLP"],
            Self::Base => vec!["Base"],
            Self::All => vec!["Base+MLP", "Base"],
        }
    }

    pub fn to_sql_filter(&self) -> &'static str {
        match self {
            Self::Full => "AND game_type = 'Base+MLP'",
            Self::Base => "AND game_type = 'Base'",
            Self::All => "AND game_type IN ('Base+MLP', 'Base')",
        }
    }

    pub fn to_string_filter(&self) -> String {
        match self {
            Self::Full => "Full".to_string(),
            Self::Base => "Base".to_string(),
            Self::All => "Full & Base".to_string(),
        }
    }
}

impl fmt::Display for GameTypeFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Self::Full => "Full",
            Self::Base => "Base",
            Self::All => "Full & Base",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for GameTypeFilter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Full" => Ok(Self::Full),
            "Base" => Ok(Self::Base),
            "Full & Base" => Ok(Self::All),
            any => Err(format!("Invalid game type filter: '{}'", any)),
        }
    }
}
