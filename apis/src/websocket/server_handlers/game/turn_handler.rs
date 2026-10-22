use super::projection::{project_committed_game, GameCommandContext};
use crate::{
    common::GameReaction,
    websocket::{
        lag_tracking::{PendingLag, ProtocolLag},
        messages::HandlerOutput,
        WebsocketData,
    },
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use db_lib::{
    game_command::{self, Command, Outcome},
    get_conn,
    models::Game,
    DbPool,
};
use hive_lib::{GameError, Piece, Position, State, Turn};
use shared_types::{GameId, RealtimeClock};
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

/// Bots send notation and the server resolves it against the replayed board, exactly as
/// `POST /api/v1/bot/games/play` does. The browser sends a resolved `Turn` because it runs
/// the engine itself; nothing else can.
pub enum TurnInput {
    Structured(Turn),
    Notation(String),
}

pub struct Mover {
    pub user_id: Uuid,
    pub socket_id: Uuid,
    pub arrived_at: DateTime<Utc>,
}

pub struct TurnHandler {
    input: TurnInput,
    expected_turn: Option<usize>,
    pool: DbPool,
    mover: Mover,
    username: String,
    game: Game,
    data: Arc<WebsocketData>,
}

impl TurnHandler {
    pub fn new(
        input: TurnInput,
        expected_turn: Option<usize>,
        game: &Game,
        username: &str,
        mover: Mover,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            mover,
            username: username.to_owned(),
            pool: pool.clone(),
            input,
            expected_turn,
            data,
        }
    }

    fn resolve(&self) -> Result<Turn> {
        let notation = match &self.input {
            TurnInput::Structured(turn) => return Ok(turn.clone()),
            TurnInput::Notation(notation) => notation,
        };
        let state = State::new_from_str(&self.game.history, &self.game.game_type)?;
        if state.turn == 0 {
            return Ok(Turn::Move(
                Piece::from_str(notation)?,
                Position::initial_spawn_position(),
            ));
        }
        let (piece, position) =
            notation
                .split_once(' ')
                .ok_or_else(|| GameError::ParsingError {
                    found: notation.clone(),
                    typ: String::from("move, expected 'piece position'"),
                })?;
        Ok(Turn::Move(
            Piece::from_str(piece)?,
            Position::from_string(position, &state.board)?,
        ))
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let turn = self.resolve()?;
        let compensation = self.compensation();
        let mut conn = get_conn(&self.pool).await?;
        let outcome = game_command::execute(
            self.game.id,
            Command::Move {
                user_id: self.mover.user_id,
                turn: turn.clone(),
                compensation: compensation.as_ref().map_or(0.0, MoveCompensation::seconds),
                expected_turn: self.expected_turn,
                arrived_at: Some(self.mover.arrived_at),
            },
            &mut conn,
        )
        .await?;
        if let (Some(compensation), Outcome::Applied { .. }) = (compensation, &outcome) {
            let refunded = self
                .game
                .compensation_refund(compensation.seconds(), self.mover.arrived_at);
            compensation.commit(refunded.as_seconds_f64(), &self.data);
        }
        let context = GameCommandContext::websocket(
            GameReaction::Turn(turn),
            self.mover.user_id,
            self.username.clone(),
        );
        let mut committed =
            project_committed_game(outcome, context, self.data.as_ref(), &mut conn).await;
        if committed.removed {
            unreachable!("a move cannot remove its game")
        }
        if let Some(reason) = committed.rejected.take() {
            committed.output.request_error = Some(reason.into());
            return Ok(committed.output);
        }
        Ok(committed.output)
    }

    fn compensation(&self) -> Option<MoveCompensation> {
        if self.game.in_untimed_opening() {
            return None;
        }
        let clock = self.game.realtime_clock().ok().flatten()?;
        Some(MoveCompensation::measure(
            &self.data,
            self.mover.user_id,
            self.mover.socket_id,
            GameId(self.game.nanoid.clone()),
            clock,
        ))
    }
}

struct MoveCompensation {
    lag: PendingLag,
    protocol: ProtocolLag,
}

impl MoveCompensation {
    fn measure(
        data: &WebsocketData,
        user_id: Uuid,
        socket_id: Uuid,
        game_id: GameId,
        clock: RealtimeClock,
    ) -> Self {
        let protocol = data.socket_lags.lag(socket_id);
        let lag = data.lags.preview(
            user_id,
            game_id,
            protocol.compensable_ms(),
            clock.base_seconds.get() as usize,
            clock.increment_seconds as usize,
        );
        Self { lag, protocol }
    }

    fn seconds(&self) -> f64 {
        self.lag.seconds
    }

    fn commit(self, refunded: f64, data: &WebsocketData) {
        data.telemetry.record_move_lag(self.protocol);
        data.lags.commit(self.lag, refunded);
    }
}

#[cfg(test)]
mod tests {
    use super::MoveCompensation;
    use crate::websocket::WebsocketData;
    use db_lib::game_command::MAX_MOVE_COMPENSATION_SECS;
    use shared_types::{GameId, RealtimeClock};
    use std::{num::NonZeroU32, time::Duration};
    use uuid::Uuid;

    fn clock(base_seconds: u32, increment_seconds: u32) -> RealtimeClock {
        RealtimeClock {
            base_seconds: NonZeroU32::new(base_seconds).expect("non-zero"),
            increment_seconds,
        }
    }

    fn game() -> GameId {
        GameId(Uuid::new_v4().to_string())
    }

    fn socket(data: &WebsocketData, rtt_ms: u64) -> Uuid {
        let socket_id = Uuid::new_v4();
        data.socket_lags.connect(socket_id, None);
        data.socket_lags
            .record_round_trip(socket_id, Duration::from_millis(rtt_ms));
        socket_id
    }

    #[test]
    fn a_move_is_compensated_for_its_own_sockets_lag() {
        let data = WebsocketData::default();
        let (user_id, game_id) = (Uuid::new_v4(), game());
        let fast_tab = socket(&data, 40);
        let slow_tab = socket(&data, 600);

        let fast =
            MoveCompensation::measure(&data, user_id, fast_tab, game_id.clone(), clock(180, 2));
        assert_eq!(fast.seconds(), 0.04);
        fast.commit(0.04, &data);
        let slow = MoveCompensation::measure(&data, user_id, slow_tab, game_id, clock(180, 2));
        assert_eq!(slow.seconds(), 0.6);
    }

    #[test]
    fn only_an_accepted_move_spends_the_budget_or_is_recorded() {
        let data = WebsocketData::default();
        let (user_id, game_id) = (Uuid::new_v4(), game());
        let tab = socket(&data, 2000);
        let bullet = clock(60, 0);
        let measure = || MoveCompensation::measure(&data, user_id, tab, game_id.clone(), bullet);

        let rejected = measure();
        assert_eq!(rejected.seconds(), 1.0);
        drop(rejected);
        let accepted = measure();
        assert_eq!(accepted.seconds(), 1.0);
        assert_eq!(data.telemetry.snapshot().lag.moves, 0);

        accepted.commit(1.0, &data);
        assert!(measure().seconds() < 1.0);
        assert_eq!(data.telemetry.snapshot().lag.moves, 1);
    }

    #[test]
    fn compensation_never_exceeds_what_the_database_accepts() {
        let data = WebsocketData::default();
        let tab = socket(&data, 10_000);

        assert_eq!(
            MoveCompensation::measure(&data, Uuid::new_v4(), tab, game(), clock(180, 2)).seconds(),
            MAX_MOVE_COMPENSATION_SECS
        );
    }
}
