use crate::{
    common::{
        game_action::GameAction,
        server_result::{GameActionResponse, ServerMessage},
    },
    responses::game::GameResponse,
    responses::user::UserResponse,
};
use anyhow::Result;
use db_lib::{models::game::Game, DbPool};
use uuid::Uuid;

use super::{
    internal_server_message::{InternalServerMessage, MessageDestination},
    messages::WsMessage,
};

pub struct JoinHandler {
    pool: DbPool,
    received_from: actix::Recipient<WsMessage>,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl JoinHandler {
    pub async fn new(
        game: Game,
        username: &str,
        user_id: Uuid,
        received_from: actix::Recipient<WsMessage>,
        pool: &DbPool,
    ) -> Self {
        Self {
            received_from,
            game,
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut messages = Vec::new();
        messages.push(InternalServerMessage {
            destination: MessageDestination::Game(self.game.nanoid.clone()),
            message: ServerMessage::Join(UserResponse::from_uuid(&self.user_id, &self.pool).await?),
        });
        messages.push(InternalServerMessage {
            destination: MessageDestination::Direct(self.received_from.clone()),
            message: ServerMessage::GameUpdate(GameActionResponse {
                game_id: self.game.nanoid.to_owned(),
                game: GameResponse::new_from_db(&self.game, &self.pool).await?,
                game_action: GameAction::Join,
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            }),
        });
        Ok(messages)
    }
}
