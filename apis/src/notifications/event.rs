use crate::i18n::*;
use chrono::{DateTime, Utc};
use shared_types::{ConversationKey, GameId, GameSpeed, NotificationCategory};
use uuid::Uuid;

use super::{payload::Push, pending::AckKey};

const DEFAULT_PUSH_TTL_SECS: u32 = 4 * 3600;

pub fn time_control_label(
    speed: GameSpeed,
    time_base: Option<i32>,
    time_increment: Option<i32>,
) -> String {
    match speed {
        GameSpeed::Untimed | GameSpeed::Correspondence => speed.to_string(),
        _ => match (time_base, time_increment) {
            (Some(base), Some(inc)) => format!("{speed} {}+{inc}", base / 60),
            _ => speed.to_string(),
        },
    }
}

fn rated_word(rated: bool) -> &'static str {
    if rated {
        "rated"
    } else {
        "casual"
    }
}

fn game_ended_phrase(outcome: GameOutcome, reason: GameEndReason, opponent: &str) -> String {
    match (outcome, reason) {
        (GameOutcome::Won, GameEndReason::Resignation) => format!("{opponent} resigned"),
        (GameOutcome::Won, GameEndReason::Timeout) => format!("{opponent} timed out"),
        (GameOutcome::Won, _) => format!("You beat {opponent}"),
        (GameOutcome::Lost, GameEndReason::Resignation) => "You resigned".to_string(),
        (GameOutcome::Lost, GameEndReason::Timeout) => "You timed out".to_string(),
        (GameOutcome::Lost, _) => format!("{opponent} beat you"),
        (GameOutcome::Drew, GameEndReason::Agreement) => {
            format!("Drew with {opponent} (agreement)")
        }
        (GameOutcome::Drew, _) => format!("Drew with {opponent}"),
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    YourTurn {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
        time_left: Option<String>,
        speed: GameSpeed,
    },
    ChallengeReceived {
        recipient: Uuid,
        challenger: String,
        challenge_nanoid: String,
        time_control: String,
        rated: bool,
    },
    GameStarted {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
        time_control: String,
        speed: GameSpeed,
    },
    GameEnded {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
        outcome: GameOutcome,
        reason: GameEndReason,
        rating_change: Option<i32>,
    },
    TournamentInvite {
        recipient: Uuid,
        tournament_name: String,
        tournament_nanoid: String,
    },
    TournamentStarted {
        recipient: Uuid,
        tournament_name: String,
        tournament_nanoid: String,
    },
    SchedulePropose {
        recipient: Uuid,
        proposer: String,
        game_nanoid: String,
        when: DateTime<Utc>,
    },
    ScheduleAccept {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
        when: DateTime<Utc>,
    },
    DirectMessage {
        recipient: Uuid,
        sender: String,
        preview: String,
        conversation_key: ConversationKey,
    },
    ChatMessage {
        recipient: Uuid,
        sender: String,
        preview: String,
        context: ChatNotifyContext,
    },
    GameControl {
        recipient: Uuid,
        actor: String,
        game_nanoid: String,
        kind: GameControlKind,
        speed: GameSpeed,
    },
    TestPush {
        recipient: Uuid,
    },
}

#[derive(Debug, Clone)]
pub enum ChatNotifyContext {
    GamePlayers {
        game_nanoid: String,
        opponent: String,
    },
    GameSpectators {
        game_nanoid: String,
    },
    Tournament {
        tournament_nanoid: String,
        tournament_name: String,
    },
    Global,
}

impl ChatNotifyContext {
    fn label(&self) -> String {
        match self {
            ChatNotifyContext::GamePlayers { opponent, .. } => format!("game vs {opponent}"),
            ChatNotifyContext::GameSpectators { .. } => "game spectators chat".to_string(),
            ChatNotifyContext::Tournament {
                tournament_name, ..
            } => format!("tournament {tournament_name}"),
            ChatNotifyContext::Global => "global chat".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameControlKind {
    DrawOffered,
    TakebackRequested,
    DrawRejected,
    TakebackAccepted,
    TakebackRejected,
}

impl GameControlKind {
    fn title(&self) -> &'static str {
        match self {
            GameControlKind::DrawOffered => "Draw offer",
            GameControlKind::TakebackRequested => "Takeback request",
            GameControlKind::DrawRejected => "Draw declined",
            GameControlKind::TakebackAccepted => "Takeback accepted",
            GameControlKind::TakebackRejected => "Takeback declined",
        }
    }

    fn action(&self) -> &'static str {
        match self {
            GameControlKind::DrawOffered => "offered a draw",
            GameControlKind::TakebackRequested => "requested a takeback",
            GameControlKind::DrawRejected => "declined the draw",
            GameControlKind::TakebackAccepted => "accepted the takeback",
            GameControlKind::TakebackRejected => "declined the takeback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameOutcome {
    Won,
    Lost,
    Drew,
}

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
            | Event::SchedulePropose { recipient, .. }
            | Event::ScheduleAccept { recipient, .. }
            | Event::DirectMessage { recipient, .. }
            | Event::ChatMessage { recipient, .. }
            | Event::GameControl { recipient, .. }
            | Event::TestPush { recipient, .. } => *recipient,
        }
    }

    pub fn category(&self) -> NotificationCategory {
        match self {
            Event::YourTurn { .. } => NotificationCategory::YourTurn,
            Event::ChallengeReceived { .. } | Event::GameStarted { .. } => {
                NotificationCategory::Challenges
            }
            Event::GameEnded { .. } => NotificationCategory::GameEnded,
            Event::TournamentInvite { .. } | Event::TournamentStarted { .. } => {
                NotificationCategory::Tournament
            }
            Event::SchedulePropose { .. } | Event::ScheduleAccept { .. } => {
                NotificationCategory::Schedules
            }
            Event::DirectMessage { .. } => NotificationCategory::Dms,
            Event::ChatMessage { .. } => NotificationCategory::GeneralChat,
            Event::GameControl { .. } => NotificationCategory::YourTurn,
            Event::TestPush { .. } => NotificationCategory::YourTurn,
        }
    }

    pub fn event_type_tag(&self) -> &'static str {
        match self {
            Event::YourTurn { .. } => "your_turn",
            Event::ChallengeReceived { .. } => "challenge",
            Event::GameStarted { .. } => "game_started",
            Event::GameEnded { .. } => "game_ended",
            Event::TournamentInvite { .. } => "tournament_invite",
            Event::TournamentStarted { .. } => "tournament_started",
            Event::SchedulePropose { .. } => "schedule_propose",
            Event::ScheduleAccept { .. } => "schedule_accept",
            Event::DirectMessage { .. } => "dm",
            Event::ChatMessage { context, .. } => match context {
                ChatNotifyContext::GamePlayers { .. }
                | ChatNotifyContext::GameSpectators { .. } => "game_chat",
                ChatNotifyContext::Tournament { .. } => "tournament_chat",
                ChatNotifyContext::Global => "global_chat",
            },
            Event::GameControl { .. } => "game_control",
            Event::TestPush { .. } => "test",
        }
    }

    pub fn ttl_secs(&self) -> u32 {
        match self {
            Event::YourTurn { speed, .. }
            | Event::GameStarted { speed, .. }
            | Event::GameControl { speed, .. } => match speed {
                GameSpeed::Bullet => 5 * 60,
                GameSpeed::Blitz => 15 * 60,
                GameSpeed::Rapid => 60 * 60,
                _ => DEFAULT_PUSH_TTL_SECS,
            },
            _ => DEFAULT_PUSH_TTL_SECS,
        }
    }

    pub fn link(&self) -> Option<String> {
        match self {
            Event::YourTurn { game_nanoid, .. }
            | Event::GameStarted { game_nanoid, .. }
            | Event::GameEnded { game_nanoid, .. }
            | Event::SchedulePropose { game_nanoid, .. }
            | Event::ScheduleAccept { game_nanoid, .. }
            | Event::GameControl { game_nanoid, .. } => {
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
            Event::DirectMessage { sender, .. } => {
                Some(format!("https://hivegame.com/message/dm/{sender}"))
            }
            Event::ChatMessage { context, .. } => Some(match context {
                ChatNotifyContext::GamePlayers { game_nanoid, .. } => {
                    format!("https://hivegame.com/message/game/{game_nanoid}/players")
                }
                ChatNotifyContext::GameSpectators { game_nanoid } => {
                    format!("https://hivegame.com/message/game/{game_nanoid}/spectators")
                }
                ChatNotifyContext::Tournament {
                    tournament_nanoid, ..
                } => format!("https://hivegame.com/message/tournament/{tournament_nanoid}"),
                ChatNotifyContext::Global => "https://hivegame.com/message/global".to_string(),
            }),
            Event::TestPush { .. } => Some("https://hivegame.com/notifications".to_string()),
        }
    }

    // Real-time turns move too fast for Discord to be anything but spam; the old
    // Busybee path skipped them entirely. Push still flows (it parks and is acked
    // when the player is on the board), so suppress only Discord here.
    pub fn suppresses_discord(&self) -> bool {
        matches!(self, Event::YourTurn { speed, .. } if speed.is_real_time())
    }

    pub fn ack_key(&self) -> Option<AckKey> {
        match self {
            Event::YourTurn { game_nanoid, .. } | Event::GameControl { game_nanoid, .. } => {
                Some(AckKey::Game(GameId(game_nanoid.clone())))
            }
            Event::DirectMessage {
                conversation_key, ..
            } => Some(AckKey::Chat(conversation_key.clone())),
            _ => None,
        }
    }

    pub fn render_push(&self, locale: Locale) -> Push {
        let (title, body) = match self {
            Event::YourTurn {
                opponent,
                time_left,
                ..
            } => {
                let body = match time_left {
                    Some(t) => td_string!(
                        locale,
                        notifications.push.your_turn_timed,
                        opponent = opponent,
                        time_left = t
                    )
                    .to_string(),
                    None => td_string!(
                        locale,
                        notifications.push.your_turn_untimed,
                        opponent = opponent
                    )
                    .to_string(),
                };
                (
                    td_string!(locale, notifications.push.your_turn_title).to_string(),
                    body,
                )
            }
            Event::ChallengeReceived {
                challenger,
                time_control,
                rated,
                ..
            } => {
                let body = if *rated {
                    td_string!(
                        locale,
                        notifications.push.challenge_rated,
                        challenger = challenger,
                        time_control = time_control
                    )
                    .to_string()
                } else {
                    td_string!(
                        locale,
                        notifications.push.challenge_casual,
                        challenger = challenger,
                        time_control = time_control
                    )
                    .to_string()
                };
                (
                    td_string!(locale, notifications.push.challenge_title).to_string(),
                    body,
                )
            }
            Event::GameStarted {
                opponent,
                time_control,
                ..
            } => (
                td_string!(locale, notifications.push.game_started_title).to_string(),
                td_string!(
                    locale,
                    notifications.push.game_started_body,
                    opponent = opponent,
                    time_control = time_control
                )
                .to_string(),
            ),
            Event::GameEnded {
                opponent,
                outcome,
                reason,
                rating_change,
                ..
            } => {
                let result = match (outcome, reason) {
                    (GameOutcome::Won, GameEndReason::Resignation) => td_string!(
                        locale,
                        notifications.push.game_ended_won_resignation,
                        opponent = opponent
                    )
                    .to_string(),
                    (GameOutcome::Won, GameEndReason::Timeout) => td_string!(
                        locale,
                        notifications.push.game_ended_won_timeout,
                        opponent = opponent
                    )
                    .to_string(),
                    (GameOutcome::Won, _) => td_string!(
                        locale,
                        notifications.push.game_ended_won,
                        opponent = opponent
                    )
                    .to_string(),
                    (GameOutcome::Lost, GameEndReason::Resignation) => {
                        td_string!(locale, notifications.push.game_ended_lost_resignation)
                            .to_string()
                    }
                    (GameOutcome::Lost, GameEndReason::Timeout) => {
                        td_string!(locale, notifications.push.game_ended_lost_timeout).to_string()
                    }
                    (GameOutcome::Lost, _) => td_string!(
                        locale,
                        notifications.push.game_ended_lost,
                        opponent = opponent
                    )
                    .to_string(),
                    (GameOutcome::Drew, GameEndReason::Agreement) => td_string!(
                        locale,
                        notifications.push.game_ended_drew_agreement,
                        opponent = opponent
                    )
                    .to_string(),
                    (GameOutcome::Drew, _) => td_string!(
                        locale,
                        notifications.push.game_ended_drew,
                        opponent = opponent
                    )
                    .to_string(),
                };
                let body = match rating_change {
                    Some(d) => format!("{result} · {d:+}"),
                    None => result,
                };
                (
                    td_string!(locale, notifications.push.game_ended_title).to_string(),
                    body,
                )
            }
            Event::TournamentInvite {
                tournament_name, ..
            } => (
                td_string!(locale, notifications.push.tournament_invite_title).to_string(),
                td_string!(
                    locale,
                    notifications.push.tournament_invite_body,
                    tournament_name = tournament_name
                )
                .to_string(),
            ),
            Event::TournamentStarted {
                tournament_name, ..
            } => (
                td_string!(locale, notifications.push.tournament_started_title).to_string(),
                td_string!(
                    locale,
                    notifications.push.tournament_started_body,
                    tournament_name = tournament_name
                )
                .to_string(),
            ),
            Event::SchedulePropose { proposer, when, .. } => {
                let when = when.format("%Y-%m-%d %H:%M UTC").to_string();
                (
                    td_string!(locale, notifications.push.schedule_propose_title).to_string(),
                    td_string!(
                        locale,
                        notifications.push.schedule_propose_body,
                        proposer = proposer,
                        when = when
                    )
                    .to_string(),
                )
            }
            Event::ScheduleAccept { opponent, when, .. } => {
                let when = when.format("%Y-%m-%d %H:%M UTC").to_string();
                (
                    td_string!(locale, notifications.push.schedule_accept_title).to_string(),
                    td_string!(
                        locale,
                        notifications.push.schedule_accept_body,
                        opponent = opponent,
                        when = when
                    )
                    .to_string(),
                )
            }
            Event::DirectMessage {
                sender, preview, ..
            } => (sender.clone(), preview.clone()),
            Event::ChatMessage {
                sender, preview, ..
            } => (sender.clone(), preview.clone()),
            Event::GameControl { actor, kind, .. } => {
                let title = match kind {
                    GameControlKind::DrawOffered => {
                        td_string!(locale, notifications.push.draw_offer_title).to_string()
                    }
                    GameControlKind::TakebackRequested => {
                        td_string!(locale, notifications.push.takeback_request_title).to_string()
                    }
                    GameControlKind::DrawRejected => {
                        td_string!(locale, notifications.push.draw_declined_title).to_string()
                    }
                    GameControlKind::TakebackAccepted => {
                        td_string!(locale, notifications.push.takeback_accepted_title).to_string()
                    }
                    GameControlKind::TakebackRejected => {
                        td_string!(locale, notifications.push.takeback_declined_title).to_string()
                    }
                };
                let body = match kind {
                    GameControlKind::DrawOffered => {
                        td_string!(locale, notifications.push.draw_offered_body, actor = actor)
                            .to_string()
                    }
                    GameControlKind::TakebackRequested => td_string!(
                        locale,
                        notifications.push.takeback_requested_body,
                        actor = actor
                    )
                    .to_string(),
                    GameControlKind::DrawRejected => {
                        td_string!(locale, notifications.push.draw_rejected_body, actor = actor)
                            .to_string()
                    }
                    GameControlKind::TakebackAccepted => td_string!(
                        locale,
                        notifications.push.takeback_accepted_body,
                        actor = actor
                    )
                    .to_string(),
                    GameControlKind::TakebackRejected => td_string!(
                        locale,
                        notifications.push.takeback_rejected_body,
                        actor = actor
                    )
                    .to_string(),
                };
                (title, body)
            }
            Event::TestPush { .. } => (
                td_string!(locale, notifications.push.test_title).to_string(),
                td_string!(locale, notifications.push.test_body).to_string(),
            ),
        };
        Push {
            title,
            body,
            link: self.link(),
            event_type: self.event_type_tag().to_string(),
            ttl_secs: self.ttl_secs(),
        }
    }

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
                time_control,
                rated,
                ..
            } => format!(
                "[New challenge](<https://hivegame.com/challenge/{challenge_nanoid}>) from {challenger} · {time_control} · {}.",
                rated_word(*rated)
            ),
            Event::GameStarted {
                opponent,
                game_nanoid,
                time_control,
                ..
            } => format!(
                "[Your game](<https://hivegame.com/game/{game_nanoid}>) vs {opponent} started · {time_control}."
            ),
            Event::GameEnded {
                opponent,
                game_nanoid,
                outcome,
                reason,
                rating_change,
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
                let change = match rating_change {
                    Some(d) => format!(" ({d:+})"),
                    None => String::new(),
                };
                format!(
                    "Your [game](<https://hivegame.com/game/{game_nanoid}>) ended — {detail}{change}."
                )
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
            Event::SchedulePropose {
                proposer,
                game_nanoid,
                when,
                ..
            } => format!(
                "[Schedule proposed](<https://hivegame.com/game/{game_nanoid}>) — {proposer} proposed {} for your game.",
                when.format("%Y-%m-%d %H:%M UTC")
            ),
            Event::ScheduleAccept {
                opponent,
                game_nanoid,
                when,
                ..
            } => format!(
                "[Schedule accepted](<https://hivegame.com/game/{game_nanoid}>) — {opponent} accepted {} for your game.",
                when.format("%Y-%m-%d %H:%M UTC")
            ),
            Event::DirectMessage { sender, preview, .. } => {
                format!("DM from {sender}: {preview}")
            }
            Event::ChatMessage {
                sender,
                preview,
                context,
                ..
            } => format!("{sender} in {}: {preview}", context.label()),
            Event::GameControl {
                actor,
                game_nanoid,
                kind,
                ..
            } => format!(
                "[Your game](<https://hivegame.com/game/{game_nanoid}>) — {actor} {}.",
                kind.action()
            ),
            Event::TestPush { .. } => "Test notification — push is working.".to_string(),
        }
    }

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
            Event::ChallengeReceived {
                challenger,
                time_control,
                rated,
                ..
            } => (
                format!("{challenger} challenged you on HiveGame"),
                format!(
                    "{challenger} sent you a {time_control} {} challenge. Open: {link}",
                    rated_word(*rated)
                ),
            ),
            Event::GameStarted {
                opponent,
                time_control,
                ..
            } => (
                format!("Game vs {opponent} started"),
                format!("Your {time_control} challenge was accepted. Open: {link}"),
            ),
            Event::GameEnded {
                opponent,
                outcome,
                reason,
                rating_change,
                ..
            } => {
                let subj_base = game_ended_phrase(*outcome, *reason, opponent);
                let subj = match rating_change {
                    Some(d) => format!("{subj_base} ({d:+})"),
                    None => subj_base,
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
            Event::SchedulePropose {
                proposer, when, ..
            } => (
                format!("{proposer} proposed a game time"),
                format!(
                    "{proposer} proposed {} for your game. Open: {link}",
                    when.format("%Y-%m-%d %H:%M UTC")
                ),
            ),
            Event::ScheduleAccept {
                opponent, when, ..
            } => (
                format!("{opponent} accepted your proposed time"),
                format!(
                    "{opponent} accepted {} for your game. Open: {link}",
                    when.format("%Y-%m-%d %H:%M UTC")
                ),
            ),
            Event::DirectMessage { sender, preview, .. } => (
                format!("New message from {sender}"),
                format!("{sender}: {preview}. Open: {link}"),
            ),
            Event::ChatMessage {
                sender,
                preview,
                context,
                ..
            } => (
                format!("New message from {sender} in {}", context.label()),
                format!("{sender}: {preview}. Open: {link}"),
            ),
            Event::GameControl { actor, kind, .. } => (
                format!("{actor} {}", kind.action()),
                format!("{} in your game. Open: {link}", kind.title()),
            ),
            Event::TestPush { .. } => (
                "HiveGame test notification".to_string(),
                "Push is working.".to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Locale;

    fn uid() -> Uuid {
        Uuid::nil()
    }

    #[test]
    fn your_turn_push_includes_opponent_and_time_left() {
        let e = Event::YourTurn {
            recipient: uid(),
            opponent: "alice".into(),
            game_nanoid: "abc".into(),
            time_left: Some("2d".into()),
            speed: GameSpeed::Correspondence,
        };
        let p = e.render_push(Locale::default());
        assert_eq!(p.title, "Your turn");
        assert!(p.body.contains("alice"), "body was {:?}", p.body);
        assert!(p.body.contains("2d"), "body was {:?}", p.body);
        assert_eq!(p.event_type, "your_turn");
        assert_eq!(p.link.as_deref(), Some("https://hivegame.com/game/abc"));
    }

    #[test]
    fn your_turn_push_without_time_left_is_plain() {
        let e = Event::YourTurn {
            recipient: uid(),
            opponent: "bob".into(),
            game_nanoid: "x".into(),
            time_left: None,
            speed: GameSpeed::Untimed,
        };
        assert_eq!(e.render_push(Locale::default()).body, "bob moved");
    }

    #[test]
    fn categories_group_related_events() {
        let started = Event::GameStarted {
            recipient: uid(),
            opponent: "a".into(),
            game_nanoid: "g".into(),
            time_control: "Blitz 5+5".into(),
            speed: GameSpeed::Blitz,
        };
        assert!(matches!(
            started.category(),
            NotificationCategory::Challenges
        ));

        let tstart = Event::TournamentStarted {
            recipient: uid(),
            tournament_name: "Cup".into(),
            tournament_nanoid: "t".into(),
        };
        assert!(matches!(
            tstart.category(),
            NotificationCategory::Tournament
        ));
    }

    #[test]
    fn game_ended_body_reflects_outcome_and_reason() {
        let won_resign = Event::GameEnded {
            recipient: uid(),
            opponent: "a".into(),
            game_nanoid: "g".into(),
            outcome: GameOutcome::Won,
            reason: GameEndReason::Resignation,
            rating_change: None,
        };
        assert!(won_resign
            .render_push(Locale::default())
            .body
            .contains("resigned"));

        let lost_timeout = Event::GameEnded {
            recipient: uid(),
            opponent: "a".into(),
            game_nanoid: "g".into(),
            outcome: GameOutcome::Lost,
            reason: GameEndReason::Timeout,
            rating_change: None,
        };
        assert!(lost_timeout
            .render_push(Locale::default())
            .body
            .contains("timed out"));

        let drew = Event::GameEnded {
            recipient: uid(),
            opponent: "a".into(),
            game_nanoid: "g".into(),
            outcome: GameOutcome::Drew,
            reason: GameEndReason::Agreement,
            rating_change: None,
        };
        assert!(drew
            .render_push(Locale::default())
            .body
            .to_lowercase()
            .contains("drew"));
    }

    #[test]
    fn game_ended_push_appends_signed_rating_change() {
        let won = Event::GameEnded {
            recipient: uid(),
            opponent: "a".into(),
            game_nanoid: "g".into(),
            outcome: GameOutcome::Won,
            reason: GameEndReason::Move,
            rating_change: Some(12),
        };
        assert!(
            won.render_push(Locale::default()).body.ends_with("· +12"),
            "body was {:?}",
            won.render_push(Locale::default()).body
        );

        let lost = Event::GameEnded {
            recipient: uid(),
            opponent: "a".into(),
            game_nanoid: "g".into(),
            outcome: GameOutcome::Lost,
            reason: GameEndReason::Move,
            rating_change: Some(-9),
        };
        assert!(lost.render_push(Locale::default()).body.ends_with("· -9"));
    }

    #[test]
    fn challenge_push_includes_time_control_and_rated() {
        let e = Event::ChallengeReceived {
            recipient: uid(),
            challenger: "c".into(),
            challenge_nanoid: "ch1".into(),
            time_control: "Blitz 5+5".into(),
            rated: true,
        };
        let body = e.render_push(Locale::default()).body;
        assert!(body.contains("Blitz 5+5"), "body was {body:?}");
        assert!(body.contains("rated"), "body was {body:?}");
    }

    #[test]
    fn challenge_link_points_at_challenge_route() {
        let e = Event::ChallengeReceived {
            recipient: uid(),
            challenger: "c".into(),
            challenge_nanoid: "ch1".into(),
            time_control: "Blitz 5+5".into(),
            rated: false,
        };
        assert_eq!(
            e.link().as_deref(),
            Some("https://hivegame.com/challenge/ch1")
        );
        assert_eq!(e.event_type_tag(), "challenge");
    }

    #[test]
    fn direct_message_link_points_at_dm_route() {
        let e = Event::DirectMessage {
            recipient: uid(),
            sender: "s".into(),
            preview: "hi".into(),
            conversation_key: ConversationKey::Direct(uid()),
        };
        assert_eq!(
            e.link().as_deref(),
            Some("https://hivegame.com/message/dm/s")
        );
    }

    #[test]
    fn direct_message_ack_key_uses_conversation_key() {
        let sender_id = Uuid::new_v4();
        let e = Event::DirectMessage {
            recipient: uid(),
            sender: "s".into(),
            preview: "hi".into(),
            conversation_key: ConversationKey::Direct(sender_id),
        };
        assert_eq!(
            e.ack_key(),
            Some(AckKey::Chat(ConversationKey::Direct(sender_id)))
        );
    }

    #[test]
    fn chat_message_category_is_general_chat() {
        let e = Event::ChatMessage {
            recipient: uid(),
            sender: "s".into(),
            preview: "hi".into(),
            context: ChatNotifyContext::Global,
        };
        assert!(matches!(e.category(), NotificationCategory::GeneralChat));
    }

    #[test]
    fn chat_message_link_per_context() {
        let players = Event::ChatMessage {
            recipient: uid(),
            sender: "s".into(),
            preview: "hi".into(),
            context: ChatNotifyContext::GamePlayers {
                game_nanoid: "g1".into(),
                opponent: "opp".into(),
            },
        };
        assert_eq!(
            players.link().as_deref(),
            Some("https://hivegame.com/message/game/g1/players")
        );
        assert_eq!(players.event_type_tag(), "game_chat");

        let spectators = Event::ChatMessage {
            recipient: uid(),
            sender: "s".into(),
            preview: "hi".into(),
            context: ChatNotifyContext::GameSpectators {
                game_nanoid: "g1".into(),
            },
        };
        assert_eq!(
            spectators.link().as_deref(),
            Some("https://hivegame.com/message/game/g1/spectators")
        );

        let tournament = Event::ChatMessage {
            recipient: uid(),
            sender: "s".into(),
            preview: "hi".into(),
            context: ChatNotifyContext::Tournament {
                tournament_nanoid: "t1".into(),
                tournament_name: "Cup".into(),
            },
        };
        assert_eq!(
            tournament.link().as_deref(),
            Some("https://hivegame.com/message/tournament/t1")
        );
        assert_eq!(tournament.event_type_tag(), "tournament_chat");

        let global = Event::ChatMessage {
            recipient: uid(),
            sender: "s".into(),
            preview: "hi".into(),
            context: ChatNotifyContext::Global,
        };
        assert_eq!(
            global.link().as_deref(),
            Some("https://hivegame.com/message/global")
        );
        assert_eq!(global.event_type_tag(), "global_chat");
    }
}
