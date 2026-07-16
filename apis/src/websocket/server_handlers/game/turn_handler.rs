use crate::websocket::{WebsocketData, WsHub};

use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    notifications::{game_end_reason_from, notify_game_ended, notify_your_turn, GameEndReason},
    websocket::messages::{
        GameFinalize,
        HandlerOutput,
        InternalServerMessage,
        MessageDestination,
        Reaction,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use hive_lib::{GameError, State, Turn};
use shared_types::{GameId, TimeMode};
use std::sync::Arc;
use uuid::Uuid;

pub struct TurnHandler {
    turn: Turn,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
}

impl TurnHandler {
    pub fn new(
        turn: Turn,
        game: &Game,
        username: &str,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
            turn,
            data,
            hub,
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        self.users_turn()?;
        let (piece, position) = match self.turn {
            Turn::Move(piece, position) => (piece, position),
            Turn::Shutout => Err(GameError::InvalidTurn {
                username: self.username.to_owned(),
                game: self.game.nanoid.to_owned(),
                turn: format!("{}", self.game.turn),
            })?,
        };
        let mut state = State::new_from_str(&self.game.history, &self.game.game_type)?;
        if let Err(err) = state.play_turn_from_position(piece, position) {
            log::warn!(
                "invalid websocket turn game={} user={} username={} db_turn={} request_turn={} error={} board=\n{}",
                self.game.nanoid,
                self.user_id,
                self.username,
                self.game.turn,
                self.turn,
                err,
                state.board,
            );
            return Err(err.into());
        }

        let comp = if self.game.time_mode == TimeMode::RealTime.to_string() {
            let ping = self.data.pings.value(self.user_id);
            let base = self.game.time_base.unwrap_or(0) as usize;
            let inc = self.game.time_increment.unwrap_or(0) as usize;
            self.data
                .lags
                .track_lag(
                    self.user_id,
                    GameId(self.game.nanoid.clone()),
                    ping,
                    base,
                    inc,
                )
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let game = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move { Ok(self.game.update_gamestate(&state, comp, tc).await?) }.scope_boxed()
            })
            .await?;

        if !game.finished {
            let opponent_id = game.not_current_player_id();
            let opponent = User::find_by_uuid(&opponent_id, &mut conn).await?;
            notify_your_turn(&game, opponent.username);
        }

        let mut messages = Vec::new();
        let next_to_move = User::find_by_uuid(&game.current_player_id, &mut conn).await?;
        let games = next_to_move.get_games_with_notifications(&mut conn).await?;
        // Batch-construct responses for the urgent list — each call is a
        // full state replay, so use the cache to share the per-game
        // allocation across this fanout and any others on the same tick.
        let mut game_responses = Vec::with_capacity(games.len());
        for g in &games {
            let resp = self.data.get_or_build_response(g, &mut conn).await?;
            game_responses.push((*resp).clone());
        }
        messages.push(InternalServerMessage {
            destination: MessageDestination::User(game.current_player_id),
            message: ServerMessage::Game(Box::new(GameUpdate::Urgent(game_responses))),
        });
        let response = self.data.get_or_build_response(&game, &mut conn).await?;
        let reactions = vec![Reaction {
            game_id: GameId(game.nanoid.to_owned()),
            white_id: game.white_id,
            black_id: game.black_id,
            gar: GameActionResponse {
                game_id: GameId(game.nanoid.to_owned()),
                game: (*response).clone(),
                game_action: GameReaction::Turn(self.turn.clone()),
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            },
        }];
        // TODO: Just add the few top games and keep them rated
        if response.time_mode == TimeMode::RealTime
            && self
                .hub
                .should_send_tv(&GameId(self.game.nanoid.clone()), game.finished)
        {
            self.data.telemetry.inc_tv_broadcast();
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Game(Box::new(GameUpdate::Tv((*response).clone()))),
            });
        };
        // If this turn finalized the game, signal the dispatcher to run
        // the cleanup hook *after* the messages above have been dispatched.
        // Eviction must happen post-dispatch so the dispatch_reaction fanout
        // above still reaches the opponent for the final move.
        let finalize_games = if game.finished {
            let reason = game_end_reason_from(&game, GameEndReason::Move);
            if let Err(e) = notify_game_ended(&game, reason, &mut conn).await {
                log::error!("notify game ended {}: {e}", game.nanoid);
            }
            let finalize = GameFinalize {
                game_id: GameId(self.game.nanoid.clone()),
                white_id: game.white_id,
                black_id: game.black_id,
            };
            messages.extend(finalize.own_game_removed_messages());
            vec![finalize]
        } else {
            Vec::new()
        };
        Ok(HandlerOutput {
            messages,
            reactions,
            finalize_games,
        })
    }

    fn users_turn(&self) -> Result<()> {
        // TODO: refactor to self.game.current_player_id == self.user_id
        if !((self.game.turn % 2 == 0 && self.game.white_id == self.user_id)
            || (self.game.turn % 2 == 1 && self.game.black_id == self.user_id))
        {
            Err(GameError::InvalidTurn {
                username: self.username.to_owned(),
                game: self.game.nanoid.to_owned(),
                turn: format!("{}", self.game.turn),
            })?;
        }
        Ok(())
    }
}
