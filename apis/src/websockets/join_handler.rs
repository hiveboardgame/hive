use crate::{
    common::{
        game_action::GameAction,
        server_result::{
            GameActionResponse, InternalServerMessage, MessageDestination, ServerMessage,
        },
    },
    functions::{games::game_response::GameStateResponse, users::user_response::UserResponse},
};
use anyhow::Result;
use db_lib::{models::game::Game, DbPool};
use uuid::Uuid;

pub struct JoinHanlder {
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl JoinHanlder {
    pub async fn new(game: Game, username: &str, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
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
            destination: MessageDestination::Direct(self.user_id),
            message: ServerMessage::GameUpdate(GameActionResponse {
                game_id: self.game.nanoid.to_owned(),
                game: GameStateResponse::new_from_db(&self.game, &self.pool).await?,
                game_action: GameAction::Join,
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            }),
        });
        Ok(messages)
    }
}
