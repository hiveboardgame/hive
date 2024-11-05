use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Takeback {
    #[default]
    Always,
    Never,
    CasualOnly,
}

impl Takeback {
    pub fn from_str_or_default(s: &str) -> Self {
        match s {
            "always" => Self::Always,
            "never" => Self::Never,
            "casual" => Self::CasualOnly,
            _ => Self::default(),
        }
    }
}
impl std::fmt::Display for Takeback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "always"),
            Self::Never => write!(f, "never"),
            Self::CasualOnly => write!(f, "casual"),
        }
    }
}
