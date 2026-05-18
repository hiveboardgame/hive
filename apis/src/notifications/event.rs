use uuid::Uuid;

use super::payload::Push;

/// Which `notification_preferences` column governs this event type.
#[derive(Debug, Clone, Copy)]
pub enum PrefField {
    YourTurn,
    Challenges,
    GameEnded,
    Tournament,
    Dms,
}

/// A user-facing notification event. Trigger sites construct an `Event` and
/// hand it to the dispatcher; the dispatcher consults the recipient's
/// `notification_preferences.{pref_field}` channel set and fans out to the
/// matching backends.
///
/// Each variant carries everything needed to render for any channel — the
/// dispatcher does no extra DB lookups for rendering. Adding a new channel
/// means adding one `render_*` method here, not touching the variants.
#[derive(Debug, Clone)]
pub enum Event {
    YourTurn {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
    },
    ChallengeReceived {
        recipient: Uuid,
        challenger: String,
        challenge_nanoid: String,
    },
    GameEnded {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
        outcome: GameOutcome,
    },
    TournamentInvite {
        recipient: Uuid,
        tournament_name: String,
        tournament_nanoid: String,
    },
    DirectMessage {
        recipient: Uuid,
        sender: String,
        preview: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOutcome {
    Won,
    Lost,
    Drew,
}

impl Event {
    pub fn recipient(&self) -> Uuid {
        match self {
            Event::YourTurn { recipient, .. }
            | Event::ChallengeReceived { recipient, .. }
            | Event::GameEnded { recipient, .. }
            | Event::TournamentInvite { recipient, .. }
            | Event::DirectMessage { recipient, .. } => *recipient,
        }
    }

    pub fn pref_field(&self) -> PrefField {
        match self {
            Event::YourTurn { .. } => PrefField::YourTurn,
            Event::ChallengeReceived { .. } => PrefField::Challenges,
            Event::GameEnded { .. } => PrefField::GameEnded,
            Event::TournamentInvite { .. } => PrefField::Tournament,
            Event::DirectMessage { .. } => PrefField::Dms,
        }
    }

    /// Short identifier used in the FCM data payload's `event_type` field
    /// so the mobile side can branch on event kind (e.g. for grouping or
    /// per-event-type UI treatment) without parsing the deep-link URL.
    pub fn event_type_tag(&self) -> &'static str {
        match self {
            Event::YourTurn { .. } => "your_turn",
            Event::ChallengeReceived { .. } => "challenge",
            Event::GameEnded { .. } => "game_ended",
            Event::TournamentInvite { .. } => "tournament_invite",
            Event::DirectMessage { .. } => "dm",
        }
    }

    /// Canonical deep-link URL the channel renderers reuse. None for events
    /// that don't have a single destination (e.g. a DM list view we don't
    /// route to today).
    pub fn link(&self) -> Option<String> {
        match self {
            Event::YourTurn { game_nanoid, .. } | Event::GameEnded { game_nanoid, .. } => {
                Some(format!("https://hivegame.com/game/{game_nanoid}"))
            }
            Event::ChallengeReceived { challenge_nanoid, .. } => {
                Some(format!("https://hivegame.com/challenge/{challenge_nanoid}"))
            }
            Event::TournamentInvite { tournament_nanoid, .. } => Some(format!(
                "https://hivegame.com/tournament/{tournament_nanoid}"
            )),
            Event::DirectMessage { .. } => None,
        }
    }

    /// FCM/APNs payload. Title + body get OS-truncated aggressively (~40 /
    /// ~120 chars on Android, similar on iOS), so prefer short over precise.
    pub fn render_push(&self) -> Push {
        let (title, body) = match self {
            Event::YourTurn { opponent, .. } => {
                ("Your turn".to_string(), format!("{opponent} moved"))
            }
            Event::ChallengeReceived { challenger, .. } => (
                "Challenge".to_string(),
                format!("{challenger} challenged you"),
            ),
            Event::GameEnded {
                opponent, outcome, ..
            } => {
                let body = match outcome {
                    GameOutcome::Won => format!("You beat {opponent}"),
                    GameOutcome::Lost => format!("{opponent} beat you"),
                    GameOutcome::Drew => format!("Drew with {opponent}"),
                };
                ("Game ended".to_string(), body)
            }
            Event::TournamentInvite { tournament_name, .. } => (
                "Tournament invite".to_string(),
                format!("Invited to {tournament_name}"),
            ),
            Event::DirectMessage { sender, preview, .. } => (sender.clone(), preview.clone()),
        };
        Push {
            title,
            body,
            link: self.link(),
            event_type: self.event_type_tag().to_string(),
        }
    }

    /// Discord message body for Busybee. Mirrors the phrasing the legacy
    /// direct-Busybee call used in `turn_handler.rs` so the migration to
    /// the unified dispatcher is behavior-preserving for `YourTurn` users
    /// who had Discord notifications via the old correspondence-game path.
    pub fn render_discord(&self) -> String {
        match self {
            Event::YourTurn {
                opponent,
                game_nanoid,
                ..
            } => format!(
                "[Your turn](<https://hivegame.com/game/{game_nanoid}>) in your game vs {opponent}."
            ),
            Event::ChallengeReceived {
                challenger,
                challenge_nanoid,
                ..
            } => format!(
                "[New challenge](<https://hivegame.com/challenge/{challenge_nanoid}>) from {challenger}."
            ),
            Event::GameEnded {
                opponent,
                game_nanoid,
                outcome,
                ..
            } => {
                let verb = match outcome {
                    GameOutcome::Won => "beat",
                    GameOutcome::Lost => "lost to",
                    GameOutcome::Drew => "drew with",
                };
                format!("Your [game](<https://hivegame.com/game/{game_nanoid}>) {verb} {opponent} has ended.")
            }
            Event::TournamentInvite {
                tournament_name,
                tournament_nanoid,
                ..
            } => format!(
                "Invited to [tournament {tournament_name}](<https://hivegame.com/tournament/{tournament_nanoid}>)."
            ),
            Event::DirectMessage { sender, preview, .. } => {
                format!("DM from {sender}: {preview}")
            }
        }
    }

    /// Email (subject, body). Stub — no SMTP backend yet, but locking the
    /// shape in here means wiring email later is a single backend addition
    /// in the dispatcher, no Event changes needed.
    pub fn render_email(&self) -> (String, String) {
        let link = self.link().unwrap_or_default();
        match self {
            Event::YourTurn {
                opponent,
                game_nanoid,
                ..
            } => (
                format!("Your turn vs {opponent}"),
                format!(
                    "{opponent} just moved in your game. Continue at https://hivegame.com/game/{game_nanoid}"
                ),
            ),
            Event::ChallengeReceived { challenger, .. } => (
                format!("{challenger} challenged you on HiveGame"),
                format!("{challenger} sent you a challenge. Open: {link}"),
            ),
            Event::GameEnded {
                opponent, outcome, ..
            } => {
                let subj = match outcome {
                    GameOutcome::Won => format!("You beat {opponent}"),
                    GameOutcome::Lost => format!("{opponent} beat you"),
                    GameOutcome::Drew => format!("Drew with {opponent}"),
                };
                (subj, format!("Review the game: {link}"))
            }
            Event::TournamentInvite {
                tournament_name, ..
            } => (
                format!("Tournament invite: {tournament_name}"),
                format!("You've been invited to {tournament_name}. Open: {link}"),
            ),
            Event::DirectMessage { sender, preview, .. } => (
                format!("New message from {sender}"),
                format!("{sender}: {preview}"),
            ),
        }
    }
}
