use std::sync::Arc;

use crate::{
    common::{GameUpdate, ServerMessage},
    responses::{GameResponse, UserResponse},
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use shared_types::GameId;
use uuid::Uuid;

pub enum BotRead {
    Game(GameId),
    OngoingGames,
    PendingGames,
    User(Uuid),
    Username(String),
}

pub struct BotReadHandler {
    read: BotRead,
    received_from: SocketTx,
    user_id: Uuid,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl BotReadHandler {
    pub fn new(
        read: BotRead,
        received_from: SocketTx,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            read,
            received_from,
            user_id,
            data,
            pool: pool.clone(),
        }
    }

    pub async fn handle(self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let message = match &self.read {
            // No ownership filter, matching `GameSelector::Specific`: bots read other
            // people's games.
            BotRead::Game(game_id) => {
                let game = Game::find_by_game_id(game_id, &mut conn).await?;
                let response = self.data.get_or_build_response(&game, &mut conn).await?;
                ServerMessage::Game(Box::new(GameUpdate::Fetched(response.clone())))
            }
            BotRead::OngoingGames => {
                let user = User::find_active_by_uuid(&self.user_id, &mut conn).await?;
                let games = user.get_ongoing_games(&mut conn).await?;
                let games = Game::settle_all(games, &mut conn).await?;
                ServerMessage::Game(Box::new(GameUpdate::Ongoing(
                    GameResponse::from_games_batch(games, &mut conn).await?,
                )))
            }
            BotRead::PendingGames => {
                let user = User::find_active_by_uuid(&self.user_id, &mut conn).await?;
                let games = user.get_games_with_notifications(&mut conn).await?;
                let games = Game::settle_all(games, &mut conn).await?;
                ServerMessage::Game(Box::new(GameUpdate::Urgent(
                    GameResponse::from_games_batch(games, &mut conn).await?,
                )))
            }
            BotRead::User(id) => {
                ServerMessage::UserProfile(UserResponse::from_uuid(id, &mut conn).await?)
            }
            BotRead::Username(name) => {
                ServerMessage::UserProfile(UserResponse::from_username(name, &mut conn).await?)
            }
        };
        Ok(HandlerOutput {
            messages: vec![InternalServerMessage {
                destination: MessageDestination::Direct(self.received_from),
                message,
            }],
            ..Default::default()
        })
    }
}
