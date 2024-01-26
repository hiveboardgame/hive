use crate::{common::server_result::ServerMessage, responses::game::GameResponse};
use anyhow::Result;
use db_lib::{models::game::Game, DbPool};
use uuid::Uuid;

use super::internal_server_message::{InternalServerMessage, MessageDestination};

#[allow(dead_code)]
pub struct GameTimeoutHandler {
    game: Game,
    nanoid: String,
    username: String,
    user_id: Uuid,
    pool: DbPool,
}

impl GameTimeoutHandler {
    pub async fn new(nanoid: &str, username: &str, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        let game = Game::find_by_nanoid(nanoid, pool).await?;
        Ok(Self {
            game,
            nanoid: nanoid.to_owned(),
            username: username.to_owned(),
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut messages = Vec::new();
        let game = self.game.check_time(&self.pool).await?;
        let game_response = GameResponse::new_from_db(&game, &self.pool).await?;
        if game.finished {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::GameTimedOut(game_response.nanoid.clone()),
            });
        }
        messages.push(InternalServerMessage {
            destination: MessageDestination::Game(self.game.nanoid.clone()),
            message: ServerMessage::GameTimeoutCheck(game_response),
        });
        Ok(messages)
    }
}
