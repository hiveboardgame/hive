use crate::{
    db_error::DbError,
    models::{Game, ScheduleOffer},
    DbConn,
};
use chrono::{DateTime, Utc};
use hive_lib::{GameControl, Turn};
use uuid::Uuid;

mod execute;
mod received_moves;

pub use execute::{real_time_flag_grace, MAX_MOVE_COMPENSATION_SECS};
pub(crate) use received_moves::awaits_move;
pub use received_moves::ReceivedMove;

#[derive(Clone, Debug)]
pub enum Command {
    Move {
        user_id: Uuid,
        turn: Turn,
        compensation: f64,
        expected_turn: Option<usize>,
        arrived_at: Option<DateTime<Utc>>,
    },
    Control {
        user_id: Uuid,
        control: GameControl,
    },
    StartReady {
        user_id: Uuid,
    },
    ReadyIntent {
        user_id: Uuid,
    },
    SettleDeadline,
    // A viewer's clock reached zero on `turn`. From the player on move on that same turn it ends
    // their own grace, as lichess flags without grace when the flagged player's own client
    // reports it, so callers send it only when that player has no other socket a move could
    // still come from.
    CheckTime {
        user_id: Uuid,
        turn: i32,
    },
    Berserk {
        user_id: Uuid,
    },
}

#[derive(Debug)]
pub enum Outcome {
    Applied {
        game: Game,
        newly_terminal: bool,
        schedule_offer_updates: Vec<ScheduleOffer>,
    },
    TimedOut {
        game: Game,
        rejected: DbError,
        schedule_offer_updates: Vec<ScheduleOffer>,
    },
    Removed {
        previous: Game,
    },
}

pub async fn execute(
    game_id: Uuid,
    command: Command,
    conn: &mut DbConn<'_>,
) -> Result<Outcome, DbError> {
    execute::execute(game_id, command, conn).await
}

#[cfg(test)]
pub(crate) use execute::execute_at;
