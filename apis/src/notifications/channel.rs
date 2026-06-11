use std::str::FromStr;

/// Delivery channel for a notification.
///
/// The DB stores these as `text[]` columns on `notification_preferences`
/// (one column per event type). Postgres CHECK constraints restrict values
/// to the three names below; we still parse defensively because a DB write
/// outside the constraint (manual psql) could in theory slip in others, and
/// we'd rather skip than panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Push,
    Email,
    Discord,
}

impl FromStr for Channel {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "push" => Ok(Channel::Push),
            "email" => Ok(Channel::Email),
            "discord" => Ok(Channel::Discord),
            _ => Err(()),
        }
    }
}

/// Convert a raw `Vec<Option<String>>` (the Diesel shape for a NOT NULL
/// Postgres `text[]` column whose elements are nullable) into a set of
/// known channels. Unknown / null entries are silently dropped.
pub fn parse_channels(raw: &[Option<String>]) -> Vec<Channel> {
    raw.iter()
        .filter_map(|s| s.as_deref().and_then(|s| s.parse().ok()))
        .collect()
}
