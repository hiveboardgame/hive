//! Server-side notification dispatch.
//!
//! Public API:
//!   notifications::init(Notifier) — call once at server startup.
//!   notifications::notify(Event)  — call from any trigger site.
//!
//! The dispatcher runs in a tokio background task and owns:
//!   - `PushBackends`: FCM client + APNs stub for the mobile push channel.
//!   - `Busybee` (referenced via `crate::websocket::busybee`): Discord channel.
//!   - Future SMTP backend for the email channel (currently stubbed).
//!
//! Per-event fan-out is driven by `notification_preferences.{event_type}` —
//! each row holds a `text[]` of channel names, validated by a DB CHECK to
//! one of `push|email|discord`. Trigger sites construct an `Event` and the
//! dispatcher reads prefs, renders per channel, and delivers — game flow
//! never waits on notification I/O.
//!
//! ## Architecture rationale
//!
//! Static singleton (matches the existing `Busybee` pattern at
//! `apis/src/websocket/busybee.rs`) rather than threading state through
//! constructors. Trigger sites stay one-line; adding a new event type means
//! one Event variant + render arms + one `notify(...)` call at the trigger.
//!
//! ## iOS push
//!
//! Paused at the project level (memory entry `ios_push_paused`). The
//! dispatcher silently skips `platform = 'apns'` push_devices rows so the
//! logs stay quiet — registration on iOS still creates rows so the day we
//! resume iOS we don't lose user opt-ins.

pub mod apns;
pub mod channel;
pub mod event;
pub mod fcm;
pub mod payload;
pub mod service;

pub use apns::ApnsNotifier;
pub use event::{Event, GameEndReason, GameOutcome};
pub use fcm::FcmNotifier;
pub use payload::Push;
pub use service::Notifier;

use std::sync::OnceLock;

/// What happened when a push delivery was attempted. Drives per-token
/// lifecycle (dead-token cleanup) and retry-or-not decisions in the
/// dispatcher.
#[derive(Debug, Clone)]
pub enum NotifyOutcome {
    Delivered,
    Retryable,
    TokenDead,
    Failed(String),
}

/// Holder of platform-specific push backends. Either field may be None if
/// the corresponding credentials weren't configured at startup. Renamed
/// from `Notifiers` to disambiguate from the top-level `Notifier` service.
pub struct PushBackends {
    pub fcm: Option<FcmNotifier>,
    pub apns: Option<ApnsNotifier>,
}

impl PushBackends {
    pub async fn send(&self, platform: &str, token: &str, push: &Push) -> NotifyOutcome {
        match platform {
            "fcm" => match &self.fcm {
                Some(n) => n.send(token, push).await,
                None => NotifyOutcome::Failed("fcm notifier not configured".into()),
            },
            "apns" => match &self.apns {
                Some(n) => n.send(token, push).await,
                None => NotifyOutcome::Failed("apns notifier not configured".into()),
            },
            other => NotifyOutcome::Failed(format!("unknown push platform: {other}")),
        }
    }
}

static NOTIFIER: OnceLock<Notifier> = OnceLock::new();

/// Install the dispatcher singleton. Call exactly once from `main.rs`
/// before `HttpServer::run`. Subsequent calls are silently ignored —
/// only the first wins. This is a deliberate fail-soft: even if something
/// double-inits, requests can still serve.
pub fn init(n: Notifier) {
    let _ = NOTIFIER.set(n);
}

/// Enqueue an event for the dispatcher. No-op (with WARN) if the
/// dispatcher wasn't initialised — keeps server tests/dev running without
/// notifications without panicking.
pub fn notify(event: Event) {
    match NOTIFIER.get() {
        Some(n) => n.enqueue(event),
        None => log::warn!("notifications::notify before init, dropping {event:?}"),
    }
}

/// Fan out `Event::GameEnded` to both players from a finalized `Game`.
/// Parses `game.game_status` into a `GameStatus::Finished(GameResult)` and
/// maps the per-player outcome (Won/Lost/Drew). The `reason` is supplied
/// by the caller because only the trigger site knows whether the game
/// ended by board state, resignation, timeout, or draw agreement.
/// No-op on `GameResult::Unknown` (rare — adjudication failure / corrupt
/// status) so we don't push a nonsense "ended" with no perspective.
/// Caller should only invoke after `game.finished == true`.
pub async fn notify_game_ended(
    game: &db_lib::models::Game,
    reason: GameEndReason,
    conn: &mut db_lib::DbConn<'_>,
) -> anyhow::Result<()> {
    use db_lib::models::User;
    use hive_lib::{Color, GameResult, GameStatus};
    use std::str::FromStr;

    let status = GameStatus::from_str(&game.game_status)
        .map_err(|e| anyhow::anyhow!("notify_game_ended: bad game_status: {e}"))?;
    let result = match status {
        GameStatus::Finished(r) => r,
        _ => return Ok(()),
    };
    let (white_outcome, black_outcome) = match result {
        GameResult::Winner(Color::White) => (GameOutcome::Won, GameOutcome::Lost),
        GameResult::Winner(Color::Black) => (GameOutcome::Lost, GameOutcome::Won),
        GameResult::Draw => (GameOutcome::Drew, GameOutcome::Drew),
        GameResult::Unknown => return Ok(()),
    };
    let white = User::find_by_uuid(&game.white_id, conn).await?;
    let black = User::find_by_uuid(&game.black_id, conn).await?;
    notify(Event::GameEnded {
        recipient: game.white_id,
        opponent: black.username.clone(),
        game_nanoid: game.nanoid.clone(),
        outcome: white_outcome,
        reason,
    });
    notify(Event::GameEnded {
        recipient: game.black_id,
        opponent: white.username,
        game_nanoid: game.nanoid.clone(),
        outcome: black_outcome,
        reason,
    });
    Ok(())
}
