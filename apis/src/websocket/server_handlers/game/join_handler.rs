use std::sync::Arc;

use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::GameResponse,
    websocket::{
        messages::{InternalServerMessage, MessageDestination, WsMessage},
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use shared_types::GameId;
use uuid::Uuid;

#[derive(Debug)]
pub struct JoinHandler {
    pool: DbPool,
    received_from: actix::Recipient<WsMessage>,
    data: Arc<WebsocketData>,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl JoinHandler {
    pub fn new(
        game: &Game,
        username: &str,
        user_id: Uuid,
        received_from: actix::Recipient<WsMessage>,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            received_from,
            game: game.to_owned(),
            user_id,
            username: username.to_owned(),
            data,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        messages.push(InternalServerMessage {
            destination: MessageDestination::Game(GameId(self.game.nanoid.clone())),
            message: ServerMessage::Join(self.user_id),
        });
        messages.push(InternalServerMessage {
            destination: MessageDestination::Direct(self.received_from.clone()),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_id: GameId(self.game.nanoid.to_owned()),
                game: GameResponse::from_model(&self.game, &mut conn).await?,
                game_action: GameReaction::Join,
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            }))),
        });
        let chat = if self.user_id == self.game.white_id || self.user_id == self.game.black_id {
            self.data.chat_storage.games_private.read().unwrap()
        } else {
            self.data.chat_storage.games_public.read().unwrap()
        };
        if let Some(messages_to_push) = chat.get(&GameId(self.game.nanoid.clone())) {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(self.user_id),
                message: ServerMessage::Chat(messages_to_push.clone()),
            });
        };
        Ok(messages)
    }
}
