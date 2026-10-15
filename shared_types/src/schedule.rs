use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleOfferStatus {
    Pending,
    Accepted,
    Declined,
    Withdrawn,
    Superseded,
    Cancelled,
}

impl ScheduleOfferStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::Declined => "declined",
            Self::Withdrawn => "withdrawn",
            Self::Superseded => "superseded",
            Self::Cancelled => "cancelled",
        }
    }
}

impl TryFrom<&str> for ScheduleOfferStatus {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "accepted" => Ok(Self::Accepted),
            "declined" => Ok(Self::Declined),
            "withdrawn" => Ok(Self::Withdrawn),
            "superseded" => Ok(Self::Superseded),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(format!("unknown schedule offer status {other:?}")),
        }
    }
}
