use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PushDeviceResponse {
    pub id: String,
    pub platform: String,
    pub last_seen: String,
    pub is_current: bool,
}

#[cfg(feature = "ssr")]
impl PushDeviceResponse {
    pub fn from_model(d: db_lib::models::PushDevice, current_endpoint: Option<&str>) -> Self {
        let is_current = current_endpoint == Some(d.device_token.as_str());
        Self {
            id: d.id.to_string(),
            platform: d.platform,
            last_seen: d.last_seen_at.format("%Y-%m-%d").to_string(),
            is_current,
        }
    }
}
