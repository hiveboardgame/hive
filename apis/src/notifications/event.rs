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
    /// Fired in `challenges/accept.rs` when a challenge becomes a live game.
    /// Recipient is always the challenger (the acceptor just clicked Accept,
    /// they know). Trigger site is responsible for the realtime / first-mover
    /// gate: realtime games push the challenger unconditionally; correspondence
    /// and untimed games push only when the challenger plays first, since the
    /// next move from the opponent already fires `YourTurn` for them.
    GameStarted {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
    },
    GameEnded {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
        outcome: GameOutcome,
        reason: GameEndReason,
    },
    TournamentInvite {
        recipient: Uuid,
        tournament_name: String,
        tournament_nanoid: String,
    },
    /// Fired from `tournaments/start.rs` when the organizer kicks the
    /// tournament off. Sent to every participant. Shares the `tournament`
    /// prefs column with `TournamentInvite` — one toggle covers the whole
    /// tournament lifecycle.
    TournamentStarted {
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

/// How a game ended. Drives the body text on GameEnded notifications so
/// the recipient knows whether it was a board outcome, a resignation,
/// a clock expiry, or a draw agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEndReason {
    Move,
    Resignation,
    Timeout,
    Agreement,
}

impl Event {
    pub fn recipient(&self) -> Uuid {
        match self {
            Event::YourTurn { recipient, .. }
            | Event::ChallengeReceived { recipient, .. }
            | Event::GameStarted { recipient, .. }
            | Event::GameEnded { recipient, .. }
            | Event::TournamentInvite { recipient, .. }
            | Event::TournamentStarted { recipient, .. }
            | Event::DirectMessage { recipient, .. } => *recipient,
        }
    }

    pub fn pref_field(&self) -> PrefField {
        match self {
            Event::YourTurn { .. } => PrefField::YourTurn,
            // GameStarted shares the `challenges` pref column: it's the
            // downstream half of the same challenges-lifecycle the user
            // opted into. Subscribing/unsubscribing is one toggle.
            Event::ChallengeReceived { .. } | Event::GameStarted { .. } => PrefField::Challenges,
            Event::GameEnded { .. } => PrefField::GameEnded,
            // TournamentStarted shares the `tournament` pref column with
            // TournamentInvite — one toggle for the whole tournament
            // lifecycle.
            Event::TournamentInvite { .. } | Event::TournamentStarted { .. } => {
                PrefField::Tournament
            }
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
            Event::GameStarted { .. } => "game_started",
            Event::GameEnded { .. } => "game_ended",
            Event::TournamentInvite { .. } => "tournament_invite",
            Event::TournamentStarted { .. } => "tournament_started",
            Event::DirectMessage { .. } => "dm",
        }
    }

    /// Canonical deep-link URL the channel renderers reuse. None for events
    /// that don't have a single destination (e.g. a DM list view we don't
    /// route to today).
    pub fn link(&self) -> Option<String> {
        match self {
            Event::YourTurn { game_nanoid, .. }
            | Event::GameStarted { game_nanoid, .. }
            | Event::GameEnded { game_nanoid, .. } => {
                Some(format!("https://hivegame.com/game/{game_nanoid}"))
            }
            Event::ChallengeReceived {
                challenge_nanoid, ..
            } => Some(format!("https://hivegame.com/challenge/{challenge_nanoid}")),
            Event::TournamentInvite {
                tournament_nanoid, ..
            }
            | Event::TournamentStarted {
                tournament_nanoid, ..
            } => Some(format!(
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
            Event::GameStarted { opponent, .. } => {
                ("Game started".to_string(), format!("vs {opponent}"))
            }
            Event::GameEnded {
                opponent,
                outcome,
                reason,
                ..
            } => {
                let body = match (outcome, reason) {
                    (GameOutcome::Won, GameEndReason::Move) => format!("You beat {opponent}"),
                    (GameOutcome::Won, GameEndReason::Resignation) => {
                        format!("{opponent} resigned")
                    }
                    (GameOutcome::Won, GameEndReason::Timeout) => format!("{opponent} timed out"),
                    (GameOutcome::Lost, GameEndReason::Move) => format!("{opponent} beat you"),
                    (GameOutcome::Lost, GameEndReason::Resignation) => "You resigned".to_string(),
                    (GameOutcome::Lost, GameEndReason::Timeout) => "You timed out".to_string(),
                    (GameOutcome::Drew, GameEndReason::Agreement) => {
                        format!("Drew with {opponent} (agreement)")
                    }
                    // Fallthrough for combos that shouldn't occur from
                    // real trigger sites (e.g. Drew + Resignation): give
                    // a sensible generic outcome string rather than panic.
                    (GameOutcome::Won, GameEndReason::Agreement) => format!("You beat {opponent}"),
                    (GameOutcome::Lost, GameEndReason::Agreement) => format!("{opponent} beat you"),
                    (GameOutcome::Drew, _) => format!("Drew with {opponent}"),
                };
                ("Game ended".to_string(), body)
            }
            Event::TournamentInvite {
                tournament_name, ..
            } => (
                "Tournament invite".to_string(),
                format!("Invited to {tournament_name}"),
            ),
            Event::TournamentStarted {
                tournament_name, ..
            } => (
                "Tournament started".to_string(),
                format!("{tournament_name} has begun"),
            ),
            Event::DirectMessage {
                sender, preview, ..
            } => (sender.clone(), preview.clone()),
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
            Event::GameStarted {
                opponent,
                game_nanoid,
                ..
            } => format!(
                "[Your game](<https://hivegame.com/game/{game_nanoid}>) vs {opponent} started."
            ),
            Event::GameEnded {
                opponent,
                game_nanoid,
                outcome,
                reason,
                ..
            } => {
                let detail = match (outcome, reason) {
                    (GameOutcome::Won, GameEndReason::Move) => format!("you beat {opponent}"),
                    (GameOutcome::Won, GameEndReason::Resignation) => {
                        format!("{opponent} resigned")
                    }
                    (GameOutcome::Won, GameEndReason::Timeout) => format!("{opponent} timed out"),
                    (GameOutcome::Lost, GameEndReason::Move) => format!("{opponent} beat you"),
                    (GameOutcome::Lost, GameEndReason::Resignation) => "you resigned".to_string(),
                    (GameOutcome::Lost, GameEndReason::Timeout) => "you timed out".to_string(),
                    (GameOutcome::Drew, GameEndReason::Agreement) => {
                        format!("you drew with {opponent} by agreement")
                    }
                    (GameOutcome::Won, GameEndReason::Agreement) => format!("you beat {opponent}"),
                    (GameOutcome::Lost, GameEndReason::Agreement) => format!("{opponent} beat you"),
                    (GameOutcome::Drew, _) => format!("you drew with {opponent}"),
                };
                format!("Your [game](<https://hivegame.com/game/{game_nanoid}>) ended — {detail}.")
            }
            Event::TournamentInvite {
                tournament_name,
                tournament_nanoid,
                ..
            } => format!(
                "Invited to [tournament {tournament_name}](<https://hivegame.com/tournament/{tournament_nanoid}>)."
            ),
            Event::TournamentStarted {
                tournament_name,
                tournament_nanoid,
                ..
            } => format!(
                "[Tournament {tournament_name}](<https://hivegame.com/tournament/{tournament_nanoid}>) has begun! Your games are ready."
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
            Event::GameStarted { opponent, .. } => (
                format!("Game vs {opponent} started"),
                format!("Your challenge was accepted. Open: {link}"),
            ),
            Event::GameEnded {
                opponent,
                outcome,
                reason,
                ..
            } => {
                let subj = match (outcome, reason) {
                    (GameOutcome::Won, GameEndReason::Move) => format!("You beat {opponent}"),
                    (GameOutcome::Won, GameEndReason::Resignation) => {
                        format!("{opponent} resigned")
                    }
                    (GameOutcome::Won, GameEndReason::Timeout) => format!("{opponent} timed out"),
                    (GameOutcome::Lost, GameEndReason::Move) => format!("{opponent} beat you"),
                    (GameOutcome::Lost, GameEndReason::Resignation) => "You resigned".to_string(),
                    (GameOutcome::Lost, GameEndReason::Timeout) => "You timed out".to_string(),
                    (GameOutcome::Drew, GameEndReason::Agreement) => {
                        format!("Drew with {opponent} (agreement)")
                    }
                    (GameOutcome::Won, GameEndReason::Agreement) => format!("You beat {opponent}"),
                    (GameOutcome::Lost, GameEndReason::Agreement) => format!("{opponent} beat you"),
                    (GameOutcome::Drew, _) => format!("Drew with {opponent}"),
                };
                (subj, format!("Review the game: {link}"))
            }
            Event::TournamentInvite {
                tournament_name, ..
            } => (
                format!("Tournament invite: {tournament_name}"),
                format!("You've been invited to {tournament_name}. Open: {link}"),
            ),
            Event::TournamentStarted {
                tournament_name, ..
            } => (
                format!("Tournament {tournament_name} started"),
                format!("{tournament_name} has begun and your games are ready. Open: {link}"),
            ),
            Event::DirectMessage { sender, preview, .. } => (
                format!("New message from {sender}"),
                format!("{sender}: {preview}"),
            ),
        }
    }
}
