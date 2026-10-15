use super::TournamentNotice;
use crate::{
    common::{game_speed_label_for_locale, schedule_slot_fragment, TournamentPairingLabel},
    i18n::*,
};
use chrono::{DateTime, Utc};
use shared_types::{GameId, GameSpeed, NotificationCategory};
use uuid::Uuid;

use super::payload::Push;

const DEFAULT_PUSH_TTL_SECS: u32 = 4 * 3600;

#[derive(Debug, Clone)]
pub struct NotificationTimeControl {
    speed: GameSpeed,
    base: Option<i32>,
    increment: Option<i32>,
}

pub fn time_control_label(
    speed: GameSpeed,
    base: Option<i32>,
    increment: Option<i32>,
) -> NotificationTimeControl {
    NotificationTimeControl {
        speed,
        base,
        increment,
    }
}

impl NotificationTimeControl {
    fn localized(&self, locale: Locale) -> String {
        let speed = game_speed_label_for_locale(locale, self.speed);
        match (self.speed, self.base, self.increment) {
            (GameSpeed::Untimed | GameSpeed::Correspondence, _, _) => speed.to_string(),
            (_, Some(base), Some(increment)) => format!("{speed} {}+{increment}", base / 60),
            _ => speed.to_string(),
        }
    }
}

fn rated_word(locale: Locale, rated: bool) -> String {
    if rated {
        td_string!(locale, game.rated).to_string()
    } else {
        td_string!(locale, game.casual).to_string()
    }
}

fn schedule_link(
    tournament_nanoid: &str,
    opponent: &str,
    slot_id: Uuid,
    open_editor: bool,
) -> String {
    let fragment = if open_editor {
        schedule_slot_fragment(slot_id)
    } else {
        format!("tournament-slot-{slot_id}")
    };
    format!("https://hivegame.com/tournament/{tournament_nanoid}/schedule/{opponent}#{fragment}")
}

fn game_ended_phrase(
    locale: Locale,
    outcome: GameOutcome,
    reason: GameEndReason,
    opponent: &str,
) -> String {
    match (outcome, reason) {
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
            td_string!(locale, notifications.push.game_ended_lost_resignation).to_string()
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
        time_control: NotificationTimeControl,
        rated: bool,
    },
    GameStarted {
        recipient: Uuid,
        opponent: String,
        game_nanoid: String,
        time_control: NotificationTimeControl,
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
    TournamentOrganizerInvite {
        recipient: Uuid,
        tournament_name: String,
        tournament_nanoid: String,
    },
    TournamentPairing {
        recipient: Uuid,
        tournament_name: String,
        game_nanoid: String,
        pairing_label: TournamentPairingLabel,
        opponent: String,
    },
    TournamentNotice {
        recipient: Uuid,
        tournament_name: String,
        tournament_nanoid: String,
        description: TournamentNotice,
        category: NotificationCategory,
    },
    TournamentStarted {
        recipient: Uuid,
        tournament_name: String,
        tournament_nanoid: String,
    },
    SchedulePropose {
        recipient: Uuid,
        proposer: String,
        tournament_nanoid: String,
        slot_id: Uuid,
        candidate_times: Vec<DateTime<Utc>>,
    },
    ScheduleAccept {
        recipient: Uuid,
        opponent: String,
        tournament_nanoid: String,
        slot_id: Uuid,
        when: DateTime<Utc>,
    },
    DirectMessage {
        recipient: Uuid,
        sender: String,
        preview: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameControlKind {
    DrawOffered,
    TakebackRequested,
    DrawRejected,
    TakebackAccepted,
    TakebackRejected,
}

impl GameControlKind {
    fn title(&self, locale: Locale) -> String {
        match self {
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
        }
    }

    fn action(&self, locale: Locale) -> String {
        match self {
            GameControlKind::DrawOffered => {
                td_string!(locale, notifications.control.offered_draw).to_string()
            }
            GameControlKind::TakebackRequested => {
                td_string!(locale, notifications.control.requested_takeback).to_string()
            }
            GameControlKind::DrawRejected => {
                td_string!(locale, notifications.control.declined_draw).to_string()
            }
            GameControlKind::TakebackAccepted => {
                td_string!(locale, notifications.control.accepted_takeback).to_string()
            }
            GameControlKind::TakebackRejected => {
                td_string!(locale, notifications.control.declined_takeback).to_string()
            }
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
            | Event::TournamentOrganizerInvite { recipient, .. }
            | Event::TournamentStarted { recipient, .. }
            | Event::TournamentPairing { recipient, .. }
            | Event::TournamentNotice { recipient, .. }
            | Event::SchedulePropose { recipient, .. }
            | Event::ScheduleAccept { recipient, .. }
            | Event::DirectMessage { recipient, .. }
            | Event::GameControl { recipient, .. }
            | Event::TestPush { recipient, .. } => *recipient,
        }
    }

    pub fn category(&self) -> NotificationCategory {
        match self {
            Event::TournamentNotice { category, .. } => *category,
            Event::YourTurn { .. } => NotificationCategory::YourTurn,
            Event::ChallengeReceived { .. } | Event::GameStarted { .. } => {
                NotificationCategory::Challenges
            }
            Event::GameEnded { .. } => NotificationCategory::GameEnded,
            Event::TournamentInvite { .. }
            | Event::TournamentOrganizerInvite { .. }
            | Event::TournamentStarted { .. }
            | Event::TournamentPairing { .. } => NotificationCategory::Tournament,
            Event::SchedulePropose { .. } | Event::ScheduleAccept { .. } => {
                NotificationCategory::Schedules
            }
            Event::DirectMessage { .. } => NotificationCategory::Dms,
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
            Event::TournamentOrganizerInvite { .. } => "tournament_organizer_invite",
            Event::TournamentStarted { .. } => "tournament_started",
            Event::TournamentNotice { .. } => "tournament_notice",
            Event::TournamentPairing { .. } => "tournament_pairing",
            Event::SchedulePropose { .. } => "schedule_propose",
            Event::ScheduleAccept { .. } => "schedule_accept",
            Event::DirectMessage { .. } => "dm",
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
            | Event::GameControl { game_nanoid, .. }
            | Event::TournamentPairing { game_nanoid, .. } => {
                Some(format!("https://hivegame.com/game/{game_nanoid}"))
            }
            Event::SchedulePropose {
                proposer,
                tournament_nanoid,
                slot_id,
                ..
            } => Some(schedule_link(tournament_nanoid, proposer, *slot_id, true)),
            Event::ScheduleAccept {
                opponent,
                tournament_nanoid,
                slot_id,
                ..
            } => Some(schedule_link(tournament_nanoid, opponent, *slot_id, false)),
            Event::ChallengeReceived {
                challenge_nanoid, ..
            } => Some(format!("https://hivegame.com/challenge/{challenge_nanoid}")),
            Event::TournamentInvite {
                tournament_nanoid, ..
            }
            | Event::TournamentOrganizerInvite {
                tournament_nanoid, ..
            }
            | Event::TournamentStarted {
                tournament_nanoid, ..
            }
            | Event::TournamentNotice {
                tournament_nanoid, ..
            } => Some(format!(
                "https://hivegame.com/tournament/{tournament_nanoid}"
            )),
            Event::DirectMessage { .. } => None,
            Event::TestPush { .. } => Some("https://hivegame.com/notifications".to_string()),
        }
    }

    // Real-time turns move too fast for Discord to be anything but spam; the old
    // Busybee path skipped them entirely. Push still flows (it parks and is acked
    // when the player is on the board), so suppress only Discord here.
    pub fn suppresses_discord(&self) -> bool {
        matches!(self, Event::YourTurn { speed, .. } if speed.is_real_time())
    }

    pub fn ack_key(&self) -> Option<GameId> {
        match self {
            Event::YourTurn { game_nanoid, .. } | Event::GameControl { game_nanoid, .. } => {
                Some(GameId(game_nanoid.clone()))
            }
            _ => None,
        }
    }

    pub fn render_push(&self, locale: Locale) -> Push {
        let (title, body) = match self {
            Event::TournamentNotice {
                tournament_name,
                description,
                ..
            } => (tournament_name.clone(), description.description(locale)),
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
                        time_control = time_control.localized(locale)
                    )
                    .to_string()
                } else {
                    td_string!(
                        locale,
                        notifications.push.challenge_casual,
                        challenger = challenger,
                        time_control = time_control.localized(locale)
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
                    time_control = time_control.localized(locale)
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
                (td_string!(locale, common.game_ended).to_string(), body)
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
            Event::TournamentOrganizerInvite {
                tournament_name, ..
            } => (
                td_string!(locale, notifications.external.organizer_title).to_string(),
                td_string!(
                    locale,
                    notifications.external.organizer_body,
                    tournament_name = tournament_name
                )
                .to_string(),
            ),
            Event::TournamentPairing {
                tournament_name,
                pairing_label,
                opponent,
                ..
            } => (
                tournament_name.clone(),
                pairing_label.description(locale, opponent),
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
            Event::SchedulePropose {
                proposer,
                candidate_times,
                ..
            } => {
                let body = match candidate_times.as_slice() {
                    [when] => {
                        let when = when.format("%Y-%m-%d %H:%M UTC").to_string();
                        td_string!(
                            locale,
                            notifications.push.schedule_propose_body,
                            proposer = proposer,
                            when = when
                        )
                        .to_string()
                    }
                    candidates => td_string!(
                        locale,
                        notifications.external.schedule_many,
                        proposer = proposer,
                        count = candidates.len()
                    )
                    .to_string(),
                };
                (
                    td_string!(locale, notifications.push.schedule_propose_title).to_string(),
                    body,
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
            open_label: td_string!(locale, common.open).to_string(),
        }
    }

    pub fn render_discord(&self, locale: Locale) -> String {
        match self {
            Event::TournamentNotice {
                tournament_name,
                tournament_nanoid,
                description,
                ..
            } => format!(
                "[{tournament_name}](<https://hivegame.com/tournament/{tournament_nanoid}>) · {}",
                description.description(locale)
            ),
            Event::YourTurn {
                opponent,
                game_nanoid,
                ..
            } => td_string!(
                locale,
                notifications.external.your_turn_discord,
                game_nanoid = game_nanoid,
                opponent = opponent
            )
            .to_string(),
            Event::ChallengeReceived {
                challenger,
                challenge_nanoid,
                time_control,
                rated,
                ..
            } => td_string!(
                locale,
                notifications.external.challenge_discord,
                challenge_nanoid = challenge_nanoid,
                challenger = challenger,
                time_control = time_control.localized(locale),
                rated = rated_word(locale, *rated)
            )
            .to_string(),
            Event::GameStarted {
                opponent,
                game_nanoid,
                time_control,
                ..
            } => td_string!(
                locale,
                notifications.external.game_started_discord,
                game_nanoid = game_nanoid,
                opponent = opponent,
                time_control = time_control.localized(locale)
            )
            .to_string(),
            Event::GameEnded {
                opponent,
                game_nanoid,
                outcome,
                reason,
                rating_change,
                ..
            } => {
                let detail = match (outcome, reason) {
                    (GameOutcome::Won, GameEndReason::Move) => td_string!(
                        locale,
                        notifications.external.game_ended_won_discord,
                        opponent = opponent
                    )
                    .to_string(),
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
                    (GameOutcome::Lost, GameEndReason::Move) => td_string!(
                        locale,
                        notifications.push.game_ended_lost,
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
                    (GameOutcome::Drew, GameEndReason::Agreement) => td_string!(
                        locale,
                        notifications.external.game_ended_draw_agreement_discord,
                        opponent = opponent
                    )
                    .to_string(),
                    (GameOutcome::Won, GameEndReason::Agreement) => td_string!(
                        locale,
                        notifications.external.game_ended_won_discord,
                        opponent = opponent
                    )
                    .to_string(),
                    (GameOutcome::Lost, GameEndReason::Agreement) => td_string!(
                        locale,
                        notifications.push.game_ended_lost,
                        opponent = opponent
                    )
                    .to_string(),
                    (GameOutcome::Drew, _) => td_string!(
                        locale,
                        notifications.external.game_ended_draw_discord,
                        opponent = opponent
                    )
                    .to_string(),
                };
                let change = match rating_change {
                    Some(d) => format!(" ({d:+})"),
                    None => String::new(),
                };
                td_string!(
                    locale,
                    notifications.external.game_ended_discord,
                    game_nanoid = game_nanoid,
                    detail = detail,
                    change = change
                )
                .to_string()
            }
            Event::TournamentInvite {
                tournament_name,
                tournament_nanoid,
                ..
            } => td_string!(
                locale,
                notifications.external.tournament_invite_discord,
                tournament_name = tournament_name,
                tournament_nanoid = tournament_nanoid
            )
            .to_string(),
            Event::TournamentOrganizerInvite {
                tournament_name,
                tournament_nanoid,
                ..
            } => td_string!(
                locale,
                notifications.external.organizer_discord,
                tournament_name = tournament_name,
                tournament_nanoid = tournament_nanoid
            )
            .to_string(),
            Event::TournamentPairing {
                tournament_name,
                pairing_label,
                opponent,
                game_nanoid,
                ..
            } => format!(
                "[{tournament_name}](<https://hivegame.com/game/{game_nanoid}>) · {}",
                pairing_label.description(locale, opponent)
            ),
            Event::TournamentStarted {
                tournament_name,
                tournament_nanoid,
                ..
            } => td_string!(
                locale,
                notifications.external.tournament_started_discord,
                tournament_name = tournament_name,
                tournament_nanoid = tournament_nanoid
            )
            .to_string(),
            Event::SchedulePropose {
                proposer,
                tournament_nanoid,
                slot_id,
                candidate_times,
                ..
            } => {
                let link = schedule_link(tournament_nanoid, proposer, *slot_id, true);
                match candidate_times.as_slice() {
                    [when] => td_string!(
                        locale,
                        notifications.external.schedule_propose_discord,
                        link = link,
                        proposer = proposer,
                        when = when.format("%Y-%m-%d %H:%M UTC").to_string()
                    )
                    .to_string(),
                    candidates => td_string!(
                        locale,
                        notifications.external.schedule_many_discord,
                        link = link,
                        proposer = proposer,
                        count = candidates.len()
                    )
                    .to_string(),
                }
            }
            Event::ScheduleAccept {
                opponent,
                tournament_nanoid,
                slot_id,
                when,
                ..
            } => {
                let link = schedule_link(tournament_nanoid, opponent, *slot_id, false);
                td_string!(
                    locale,
                    notifications.external.schedule_accept_discord,
                    link = link,
                    opponent = opponent,
                    when = when.format("%Y-%m-%d %H:%M UTC").to_string()
                )
                .to_string()
            }
            Event::DirectMessage {
                sender, preview, ..
            } => td_string!(
                locale,
                notifications.external.direct_message_discord,
                sender = sender,
                preview = preview
            )
            .to_string(),
            Event::GameControl {
                actor,
                game_nanoid,
                kind,
                ..
            } => td_string!(
                locale,
                notifications.external.game_control_discord,
                game_nanoid = game_nanoid,
                actor = actor,
                action = kind.action(locale)
            )
            .to_string(),
            Event::TestPush { .. } => td_string!(locale, notifications.push.test_body).to_string(),
        }
    }

    pub fn render_email(&self, locale: Locale) -> (String, String) {
        let link = self.link().unwrap_or_default();
        match self {
            Event::TournamentNotice {
                tournament_name,
                description,
                ..
            } => (
                tournament_name.clone(),
                td_string!(
                    locale,
                    notifications.external.open_description,
                    description = description.description(locale),
                    link = link
                )
                .to_string(),
            ),
            Event::YourTurn {
                opponent,
                game_nanoid,
                ..
            } => (
                td_string!(
                    locale,
                    notifications.external.your_turn_subject,
                    opponent = opponent
                )
                .to_string(),
                td_string!(
                    locale,
                    notifications.external.your_turn_email,
                    opponent = opponent,
                    game_nanoid = game_nanoid
                )
                .to_string(),
            ),
            Event::ChallengeReceived {
                challenger,
                time_control,
                rated,
                ..
            } => (
                td_string!(
                    locale,
                    notifications.external.challenge_subject,
                    challenger = challenger
                )
                .to_string(),
                td_string!(
                    locale,
                    notifications.external.challenge_email,
                    challenger = challenger,
                    time_control = time_control.localized(locale),
                    rated = rated_word(locale, *rated),
                    link = link
                )
                .to_string(),
            ),
            Event::GameStarted {
                opponent,
                time_control,
                ..
            } => (
                td_string!(
                    locale,
                    notifications.external.game_started_subject,
                    opponent = opponent
                )
                .to_string(),
                td_string!(
                    locale,
                    notifications.external.game_started_email,
                    time_control = time_control.localized(locale),
                    link = link
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
                let subj_base = game_ended_phrase(locale, *outcome, *reason, opponent);
                let subj = match rating_change {
                    Some(d) => format!("{subj_base} ({d:+})"),
                    None => subj_base,
                };
                (
                    subj,
                    td_string!(locale, notifications.external.game_ended_email, link = link)
                        .to_string(),
                )
            }
            Event::TournamentInvite {
                tournament_name, ..
            } => (
                td_string!(
                    locale,
                    notifications.external.tournament_invite_subject,
                    tournament_name = tournament_name
                )
                .to_string(),
                td_string!(
                    locale,
                    notifications.external.tournament_invite_email,
                    tournament_name = tournament_name,
                    link = link
                )
                .to_string(),
            ),
            Event::TournamentOrganizerInvite {
                tournament_name, ..
            } => (
                td_string!(
                    locale,
                    notifications.external.organizer_subject,
                    tournament_name = tournament_name
                )
                .to_string(),
                td_string!(
                    locale,
                    notifications.external.organizer_email,
                    tournament_name = tournament_name,
                    link = link
                )
                .to_string(),
            ),
            Event::TournamentPairing {
                tournament_name,
                pairing_label,
                opponent,
                ..
            } => (
                tournament_name.clone(),
                td_string!(
                    locale,
                    notifications.external.open_description,
                    description = pairing_label.description(locale, opponent),
                    link = link
                )
                .to_string(),
            ),
            Event::TournamentStarted {
                tournament_name, ..
            } => (
                td_string!(
                    locale,
                    notifications.external.tournament_started_subject,
                    tournament_name = tournament_name
                )
                .to_string(),
                td_string!(
                    locale,
                    notifications.external.tournament_started_email,
                    tournament_name = tournament_name,
                    link = link
                )
                .to_string(),
            ),
            Event::SchedulePropose {
                proposer,
                candidate_times,
                ..
            } => match candidate_times.as_slice() {
                [when] => (
                    td_string!(
                        locale,
                        notifications.external.schedule_propose_subject,
                        proposer = proposer
                    )
                    .to_string(),
                    td_string!(
                        locale,
                        notifications.external.schedule_propose_email,
                        proposer = proposer,
                        when = when.format("%Y-%m-%d %H:%M UTC").to_string(),
                        link = link
                    )
                    .to_string(),
                ),
                candidates => (
                    td_string!(
                        locale,
                        notifications.external.schedule_many_subject,
                        proposer = proposer
                    )
                    .to_string(),
                    td_string!(
                        locale,
                        notifications.external.schedule_many_email,
                        proposer = proposer,
                        count = candidates.len(),
                        link = link
                    )
                    .to_string(),
                ),
            },
            Event::ScheduleAccept { opponent, when, .. } => (
                td_string!(
                    locale,
                    notifications.external.schedule_accept_subject,
                    opponent = opponent
                )
                .to_string(),
                td_string!(
                    locale,
                    notifications.external.schedule_accept_email,
                    opponent = opponent,
                    when = when.format("%Y-%m-%d %H:%M UTC").to_string(),
                    link = link
                )
                .to_string(),
            ),
            Event::DirectMessage {
                sender, preview, ..
            } => (
                td_string!(
                    locale,
                    notifications.external.direct_message_subject,
                    sender = sender
                )
                .to_string(),
                format!("{sender}: {preview}"),
            ),
            Event::GameControl { actor, kind, .. } => (
                td_string!(
                    locale,
                    notifications.external.game_control_subject,
                    actor = actor,
                    action = kind.action(locale)
                )
                .to_string(),
                td_string!(
                    locale,
                    notifications.external.game_control_email,
                    title = kind.title(locale),
                    link = link
                )
                .to_string(),
            ),
            Event::TestPush { .. } => (
                td_string!(locale, notifications.external.test_subject).to_string(),
                td_string!(locale, notifications.external.test_email).to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

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
            time_control: time_control_label(GameSpeed::Blitz, Some(300), Some(5)),
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
            time_control: time_control_label(GameSpeed::Blitz, Some(300), Some(5)),
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
            time_control: time_control_label(GameSpeed::Blitz, Some(300), Some(5)),
            rated: false,
        };
        assert_eq!(
            e.link().as_deref(),
            Some("https://hivegame.com/challenge/ch1")
        );
        assert_eq!(e.event_type_tag(), "challenge");
    }

    #[test]
    fn direct_message_has_no_link() {
        let e = Event::DirectMessage {
            recipient: uid(),
            sender: "s".into(),
            preview: "hi".into(),
        };
        assert!(e.link().is_none());
    }

    #[test]
    fn multi_candidate_proposals_are_rendered_as_a_choice_across_channels() {
        let first = Utc::now() + Duration::hours(1);
        let second = Utc::now() + Duration::hours(2);
        let event = Event::SchedulePropose {
            recipient: uid(),
            proposer: String::from("alice"),
            tournament_nanoid: String::from("cup"),
            slot_id: uid(),
            candidate_times: vec![first, second],
        };

        let push = event.render_push(Locale::default()).body;
        let discord = event.render_discord(Locale::default());
        let email = event.render_email(Locale::default()).1;
        for rendered in [push, discord, email] {
            assert!(rendered.contains("2 times"), "rendered was {rendered:?}");
            assert!(
                !rendered.contains(&first.format("%Y-%m-%d %H:%M UTC").to_string()),
                "rendered was {rendered:?}"
            );
        }
    }
}
