use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Push {
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub event_type: String,
    pub ttl_secs: u32,
}
