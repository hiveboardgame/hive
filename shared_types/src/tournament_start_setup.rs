use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The exclusive, fixed-duration review of an accepted elimination roster.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TournamentStartSetup {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub opened_at: DateTime<Utc>,
    pub seeded_players: Vec<Uuid>,
}

impl TournamentStartSetup {
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.opened_at + Duration::minutes(15)
    }

    pub fn active_at(&self, now: DateTime<Utc>) -> bool {
        now < self.expires_at()
    }
}
