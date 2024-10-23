use super::start::StartHandler;
use super::{
    control_handler::GameControlHandler, join_handler::JoinHandler,
    timeout_handler::TimeoutHandler, turn_handler::TurnHandler,
};
use crate::websocket::lag_tracking::Lags;
use crate::websocket::lag_tracking::Pings;
use crate::websocket::messages::InternalServerMessage;
use crate::websocket::messages::WsMessage;
use crate::websocket::tournament_game_start::TournamentGameStart;
use crate::{common::GameAction, websocket::chat::Chats};
use anyhow::Result;
use db_lib::get_conn;
use db_lib::{models::Game, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use hive_lib::{GameError, GameStatus};
use shared_types::GameId;
use std::str::FromStr;
use uuid::Uuid;
pub struct GameActionHandler {
    game_action: GameAction,
    game: Game,
    pool: DbPool,
    user_id: Uuid,
    received_from: actix::Recipient<WsMessage>,
    chat_storage: actix_web::web::Data<Chats>,
    game_start: actix_web::web::Data<TournamentGameStart>,
    pings: actix_web::web::Data<Pings>,
    lags: actix_web::web::Data<Lags>,
    username: String,
}

impl GameActionHandler {
    pub async fn new(
        game_id: &GameId,
        game_action: GameAction,
        user_details: (&str, Uuid),
        received_from: actix::Recipient<WsMessage>,
        chat_storage: actix_web::web::Data<Chats>,
        game_start: actix_web::web::Data<TournamentGameStart>,
        pings: actix_web::web::Data<Pings>,
        lags: actix_web::web::Data<Lags>,
        pool: &DbPool,
    ) -> Result<Self> {
        let (username, user_id) = user_details;
        let mut connection = get_conn(pool).await?;

        let game = connection
            .transaction::<_, anyhow::Error, _>(move |conn| {
                // find_by_game_id automatically times the game out if needed
                async move { Ok(Game::find_by_game_id(game_id, conn).await?) }.scope_boxed()
            })
            .await?;

        Ok(Self {
            pool: pool.clone(),
            game,
            username: username.to_owned(),
            game_action,
            received_from,
            chat_storage,
            game_start,
            user_id,
            pings,
            lags,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let messages = match self.game_action.clone() {
            GameAction::CheckTime => {
                TimeoutHandler::new(&self.game, &self.username, self.user_id, &self.pool)
                    .handle()
                    .await?
            }
            GameAction::Turn(turn) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                TurnHandler::new(
                    turn,
                    &self.game,
                    &self.username,
                    self.user_id,
                    self.pings.clone(),
                    self.lags.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Control(control) => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                GameControlHandler::new(
                    &control,
                    &self.game,
                    &self.username,
                    self.user_id,
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Join => {
                JoinHandler::new(
                    &self.game,
                    &self.username,
                    self.user_id,
                    self.received_from.clone(),
                    self.chat_storage.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Start => {
                self.ensure_not_finished()?;
                self.ensure_user_is_player()?;
                StartHandler::new(
                    &self.game,
                    self.user_id,
                    self.username.clone(),
                    self.game_start.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
        };
        Ok(messages)
    }

    fn ensure_not_finished(&self) -> Result<()> {
        if let GameStatus::Finished(_) = GameStatus::from_str(&self.game.game_status).unwrap() {
            Err(GameError::GameIsOver {
                username: self.username.to_owned(),
                game: self.game.nanoid.to_owned(),
            })?;
        }
        Ok(())
    }

    fn ensure_user_is_player(&self) -> Result<()> {
        if !self.game.user_is_player(self.user_id) {
            Err(GameError::NotPlayer {
                username: self.username.to_owned(),
                game: self.game.nanoid.clone(),
            })?;
        }
        Ok(())
    }
}
