use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::GameResponse,
    websocket::{
        messages::{
            GameFinalize,
            HandlerOutput,
            InternalServerMessage,
            MessageDestination,
            Reaction,
        },
        WebsocketData,
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, User},
    DbConn,
    DbPool,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use hive_lib::{GameControl, GameError};
use shared_types::{Conclusion, GameId, TimeMode};
use std::sync::Arc;
use uuid::Uuid;

pub struct GameControlHandler {
    control: GameControl,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
}

impl GameControlHandler {
    pub fn new(
        control: &GameControl,
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
            control: control.to_owned(),
            data,
            hub,
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        // the GC can be played this turn
        self.ensure_gc_allowed_for_turn()?;
        // the GC(color) matches the user color
        self.ensure_gc_color()?;
        // the gc hasn't been sent previously
        self.ensure_fresh_game_control()?;

        let game = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move { self.match_control(tc).await }.scope_boxed()
            })
            .await?;
        let game_response = self.game_response_for_control(&game, &mut conn).await?;
        let game_action = Self::reaction_for_control_result(&game, self.control);

        let reactions = vec![Reaction {
            game_id: GameId(self.game.nanoid.clone()),
            white_id: self.game.white_id,
            black_id: self.game.black_id,
            gar: GameActionResponse {
                game_id: GameId(self.game.nanoid.to_owned()),
                game: (*game_response).clone(),
                game_action,
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            },
        }];
        match self.control {
            GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
                let current_user = User::find_by_uuid(&game.current_player_id, &mut conn).await?;
                let games = current_user.get_games_with_notifications(&mut conn).await?;
                let game_responses = GameResponse::from_games_batch(games, &mut conn).await?;
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(game.current_player_id),
                    message: ServerMessage::Game(Box::new(GameUpdate::Urgent(game_responses))),
                });
            }
            _ => {}
        }
        if game_response.time_mode == TimeMode::RealTime
            && self
                .hub
                .should_send_tv(&GameId(self.game.nanoid.clone()), game.finished)
        {
            self.data.telemetry.inc_tv_broadcast();
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Game(Box::new(GameUpdate::Tv((*game_response).clone()))),
            });
        };
        // Signal the dispatcher to run finalization after dispatch — see the
        // matching comment in turn_handler.rs for the race we're avoiding.
        let finalize_games = if game.finished {
            let finalize = GameFinalize {
                game_id: GameId(self.game.nanoid.clone()),
                white_id: self.game.white_id,
                black_id: self.game.black_id,
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
            subscriptions: Vec::new(),
        })
    }

    fn reaction_for_control_result(game: &Game, control: GameControl) -> GameReaction {
        if game.finished && game.conclusion == Conclusion::Timeout.to_string() {
            return GameReaction::TimedOut;
        }
        GameReaction::Control(control)
    }

    fn ensure_fresh_game_control(&self) -> Result<()> {
        if let Some(last) = self.game.last_game_control() {
            if last == self.control {
                Err(GameError::GcAlreadyPresent {
                    gc: self.control.to_string(),
                    game: self.game.nanoid.to_owned(),
                    turn: format!("{}", self.game.turn),
                })?
            }
        }
        Ok(())
    }

    fn ensure_previous_gc_present(&self) -> Result<()> {
        let opposite_color = self.control.color().opposite_color();
        let should_be_gc = match self.control {
            GameControl::TakebackAccept(_) => GameControl::TakebackRequest(opposite_color),
            GameControl::TakebackReject(_) => GameControl::TakebackRequest(opposite_color),
            GameControl::DrawReject(_) => GameControl::DrawOffer(opposite_color),
            GameControl::DrawAccept(_) => GameControl::DrawOffer(opposite_color),
            _ => unreachable!(),
        };
        if let Some(last_gc) = self.game.last_game_control() {
            if last_gc == should_be_gc {
                return Ok(());
            }
        }
        Err(GameError::InvalidGc {
            gc: self.control.to_string(),
            game: self.game.nanoid.to_owned(),
            turn: format!("{}", self.game.turn),
        })?
    }

    async fn handle_takeback_request(&self, conn: &mut DbConn<'_>) -> Result<Game> {
        let game = self.game.write_game_control(&self.control, conn).await?;
        Ok(game)
    }

    async fn game_response_for_control(
        &self,
        game: &Game,
        conn: &mut DbConn<'_>,
    ) -> Result<Arc<GameResponse>> {
        if matches!(self.control, GameControl::Abort(_)) {
            let game_id = GameId(game.nanoid.clone());
            self.data.game_response_cache.remove(&game_id);
            self.data.telemetry.inc_from_model();
            return Ok(Arc::new(GameResponse::from_model(game, conn).await?));
        }

        self.data.get_or_build_response(game, conn).await
    }

    async fn handle_takeback_accept(&self, conn: &mut DbConn<'_>) -> Result<Game> {
        self.ensure_previous_gc_present()?;
        let game = self.game.accept_takeback(&self.control, conn).await?;
        Ok(game)
    }

    async fn handle_resign(&self, conn: &mut DbConn<'_>) -> Result<Game> {
        let game = self.game.resign(&self.control, conn).await?;
        Ok(game)
    }

    async fn handle_abort(&self, conn: &mut DbConn<'_>) -> Result<()> {
        if self.game.tournament_id.is_some() {
            Err(GameError::TournamentAbort)?
        }
        let game_id = GameId(self.game.nanoid.clone());
        // Guard pattern: a panic between marking-pending and delete-committing
        // would otherwise leak the marker until the heartbeat sweep. Drop
        // clears on the unwind path; `disarm()` only runs after the commit.
        let guard = self
            .hub
            .arm_pending_delete(game_id, self.game.white_id, self.game.black_id);
        self.game.delete(conn).await?;
        guard.disarm();
        Ok(())
    }

    async fn handle_draw_reject(&self, conn: &mut DbConn<'_>) -> Result<Game> {
        self.ensure_previous_gc_present()?;
        let game = self.game.write_game_control(&self.control, conn).await?;
        Ok(game)
    }

    async fn handle_draw_offer(&self, conn: &mut DbConn<'_>) -> Result<Game> {
        let game = self.game.write_game_control(&self.control, conn).await?;
        Ok(game)
    }

    async fn handle_draw_accept(&self, conn: &mut DbConn<'_>) -> Result<Game> {
        self.ensure_previous_gc_present()?;
        let game = self.game.accept_draw(&self.control, conn).await?;
        Ok(game)
    }

    async fn handle_takeback_reject(&self, conn: &mut DbConn<'_>) -> Result<Game> {
        self.ensure_previous_gc_present()?;
        let game = self.game.write_game_control(&self.control, conn).await?;
        Ok(game)
    }

    async fn match_control(&self, conn: &mut DbConn<'_>) -> Result<Game> {
        Ok(match self.control {
            GameControl::Abort(_) => {
                let mut game = self.game.clone();
                self.handle_abort(conn).await?;
                game.finished = true;
                game
            }
            GameControl::Resign(_) => self.handle_resign(conn).await?,
            GameControl::DrawOffer(_) => self.handle_draw_offer(conn).await?,
            GameControl::DrawAccept(_) => self.handle_draw_accept(conn).await?,
            GameControl::DrawReject(_) => self.handle_draw_reject(conn).await?,
            GameControl::TakebackRequest(_) => self.handle_takeback_request(conn).await?,
            GameControl::TakebackAccept(_) => self.handle_takeback_accept(conn).await?,
            GameControl::TakebackReject(_) => self.handle_takeback_reject(conn).await?,
        })
    }

    fn ensure_gc_allowed_for_turn(&self) -> Result<()> {
        if let Some(_color) = self.game.user_color(self.user_id) {
            if !self.control.allowed_on_turn(self.game.turn) {
                Err(GameError::InvalidGc {
                    gc: self.control.to_string(),
                    game: self.game.nanoid.to_owned(),
                    turn: format!("{}", self.game.turn),
                })?
            }
            return Ok(());
        }
        Err(GameError::InvalidGc {
            gc: self.control.to_string(),
            game: self.game.nanoid.to_owned(),
            turn: format!("{}", self.game.turn),
        })?
    }

    fn ensure_gc_color(&self) -> Result<()> {
        if let Some(color) = self.game.user_color(self.user_id) {
            if color == self.control.color() {
                return Ok(());
            }
        }
        Err(GameError::InvalidGc {
            gc: self.control.to_string(),
            game: self.game.nanoid.to_owned(),
            turn: format!("{}", self.game.turn),
        })?
    }
}
