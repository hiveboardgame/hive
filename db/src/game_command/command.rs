use crate::{
    db_error::DbError,
    models::{Game, ScheduleOffer},
};
use chrono::{DateTime, Utc};
use hive_lib::{GameControl, Turn};
use uuid::Uuid;

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
    // The player on move reporting their own clock for the current turn ends their grace, as on
    // lichess. Callers send it rather than SettleDeadline only when no other socket of theirs
    // could still send a move.
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
