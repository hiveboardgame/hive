use super::{
    event::{Event, GameControlKind, GameEndReason, GameOutcome},
    payload::Push,
    service::Notifier,
    web_push::WebPushNotifier,
};
use dashmap::{mapref::entry::Entry, DashMap};
use db_lib::models::User;
use hive_lib::{Color, GameResult, GameStatus};
use shared_types::{Conclusion, TimeMode};
use std::{
    str::FromStr,
    sync::{LazyLock, OnceLock},
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
pub enum NotifyOutcome {
    Delivered,
    Retryable,
    TokenDead,
    Failed(String),
}

pub struct PushBackends {
    pub web: Option<WebPushNotifier>,
}

impl PushBackends {
    pub async fn send(&self, device: &db_lib::models::PushDevice, push: &Push) -> NotifyOutcome {
        match (device.platform.as_str(), &self.web) {
            ("web", Some(n)) => match (&device.p256dh, &device.auth) {
                (Some(p256dh), Some(auth)) => {
                    n.send(&device.device_token, p256dh, auth, push).await
                }
                _ => NotifyOutcome::TokenDead,
            },
            ("web", None) => NotifyOutcome::Failed("web push notifier not configured".into()),
            _ => NotifyOutcome::TokenDead,
        }
    }
}

static NOTIFIER: OnceLock<Notifier> = OnceLock::new();

pub fn init(n: Notifier) {
    let _ = NOTIFIER.set(n);
}

pub fn notify(event: Event) {
    match NOTIFIER.get() {
        Some(n) => n.enqueue(event),
        None => log::warn!("notifications::notify before init, dropping {event:?}"),
    }
}

static GAME_ENDED_FIRED: LazyLock<DashMap<String, Instant>> = LazyLock::new(DashMap::new);

const GAME_ENDED_DEDUP_TTL: Duration = Duration::from_secs(300);

fn game_ended_unfired(nanoid: &str) -> bool {
    match GAME_ENDED_FIRED.entry(nanoid.to_string()) {
        Entry::Occupied(mut e) if e.get().elapsed() >= GAME_ENDED_DEDUP_TTL => {
            e.insert(Instant::now());
            true
        }
        Entry::Occupied(_) => false,
        Entry::Vacant(e) => {
            e.insert(Instant::now());
            true
        }
    }
}

pub fn sweep_game_ended_dedup() {
    GAME_ENDED_FIRED.retain(|_, fired| fired.elapsed() < GAME_ENDED_DEDUP_TTL);
}

pub fn game_end_reason_from(game: &db_lib::models::Game, fallback: GameEndReason) -> GameEndReason {
    match Conclusion::from_str(&game.conclusion) {
        Ok(Conclusion::Timeout) => GameEndReason::Timeout,
        Ok(Conclusion::Resigned) => GameEndReason::Resignation,
        Ok(Conclusion::Draw) => GameEndReason::Agreement,
        _ => fallback,
    }
}

pub async fn notify_game_ended(
    game: &db_lib::models::Game,
    reason: GameEndReason,
    conn: &mut db_lib::DbConn<'_>,
) -> anyhow::Result<()> {
    notify_game_ended_excluding(game, reason, None, conn).await
}

pub async fn notify_game_ended_excluding(
    game: &db_lib::models::Game,
    reason: GameEndReason,
    exclude: Option<uuid::Uuid>,
    conn: &mut db_lib::DbConn<'_>,
) -> anyhow::Result<()> {
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
    let users = User::find_by_uuids(&[game.white_id, game.black_id], conn).await?;
    if !game_ended_unfired(&game.nanoid) {
        return Ok(());
    }
    let name = |id| {
        users
            .iter()
            .find(|u| u.id == id)
            .map(|u| u.username.clone())
            .unwrap_or_default()
    };
    let (white_name, black_name) = (name(game.white_id), name(game.black_id));
    let round = |c: Option<f64>| c.map(|v| v.round() as i32);
    if exclude != Some(game.white_id) {
        notify(Event::GameEnded {
            recipient: game.white_id,
            opponent: black_name,
            game_nanoid: game.nanoid.clone(),
            outcome: white_outcome,
            reason,
            rating_change: round(game.white_rating_change),
        });
    }
    if exclude != Some(game.black_id) {
        notify(Event::GameEnded {
            recipient: game.black_id,
            opponent: white_name,
            game_nanoid: game.nanoid.clone(),
            outcome: black_outcome,
            reason,
            rating_change: round(game.black_rating_change),
        });
    }
    Ok(())
}

pub fn notify_game_control(
    recipient: uuid::Uuid,
    actor: String,
    game_nanoid: String,
    kind: GameControlKind,
    speed: shared_types::GameSpeed,
) {
    notify(Event::GameControl {
        recipient,
        actor,
        game_nanoid,
        kind,
        speed,
    });
}

pub fn notify_your_turn(game: &db_lib::models::Game, opponent: String) {
    let time_left = match TimeMode::from_str(&game.time_mode) {
        Ok(TimeMode::Untimed) => None,
        _ => Some(game.str_time_left_for_player(game.current_player_id)),
    };
    notify(Event::YourTurn {
        recipient: game.current_player_id,
        opponent,
        game_nanoid: game.nanoid.clone(),
        time_left,
        speed: shared_types::GameSpeed::from_base_increment(game.time_base, game.time_increment),
    });
}
