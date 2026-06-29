use shared_types::{CHANNEL_DISCORD, CHANNEL_EMAIL, CHANNEL_PUSH};
use std::str::FromStr;

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
            CHANNEL_PUSH => Ok(Channel::Push),
            CHANNEL_EMAIL => Ok(Channel::Email),
            CHANNEL_DISCORD => Ok(Channel::Discord),
            _ => Err(()),
        }
    }
}

pub fn parse_channels(raw: &[Option<String>]) -> Vec<Channel> {
    let mut seen = std::collections::HashSet::new();
    raw.iter()
        .filter_map(|s| s.as_deref().and_then(|s| s.parse().ok()))
        .filter(|c| seen.insert(*c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_channels, Channel};
    use std::str::FromStr;

    #[test]
    fn parses_known_channel_names() {
        assert_eq!(Channel::from_str("push"), Ok(Channel::Push));
        assert_eq!(Channel::from_str("email"), Ok(Channel::Email));
        assert_eq!(Channel::from_str("discord"), Ok(Channel::Discord));
        assert!(Channel::from_str("sms").is_err());
        assert!(Channel::from_str("").is_err());
    }

    #[test]
    fn parse_channels_drops_unknown_and_null_preserving_order() {
        let raw = vec![
            Some("push".to_string()),
            None,
            Some("bogus".to_string()),
            Some("discord".to_string()),
        ];
        assert_eq!(parse_channels(&raw), vec![Channel::Push, Channel::Discord]);
    }

    #[test]
    fn parse_channels_dedups_preserving_first() {
        let raw = vec![
            Some("push".to_string()),
            Some("discord".to_string()),
            Some("push".to_string()),
        ];
        assert_eq!(parse_channels(&raw), vec![Channel::Push, Channel::Discord]);
    }

    #[test]
    fn parse_channels_empty_is_empty() {
        assert!(parse_channels(&[]).is_empty());
        assert!(parse_channels(&[None, Some("nope".to_string())]).is_empty());
    }
}
