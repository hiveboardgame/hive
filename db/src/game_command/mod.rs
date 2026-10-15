use crate::{
    db_error::DbError,
    models::{Game, ScheduleOffer},
    DbConn,
};
use hive_lib::{GameControl, Turn};
use uuid::Uuid;

mod execute;

#[derive(Clone, Debug)]
pub enum Command {
    Move {
        user_id: Uuid,
        turn: Turn,
        compensation: f64,
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
