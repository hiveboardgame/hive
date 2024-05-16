use crate::{
    common::{
        GameReaction, {GameActionResponse, GameUpdate, ServerMessage},
    },
    responses::GameResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{models::Game, DbPool};
use uuid::Uuid;

pub struct TimeoutHandler {
    game: Game,
    username: String,
    user_id: Uuid,
    pool: DbPool,
}

impl TimeoutHandler {
    pub fn new(game: Game, username: &str, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            game,
            username: username.to_owned(),
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut messages = Vec::new();
        let game = self.game.check_time(&self.pool).await?;
        let game_response = GameResponse::new_from_db(&game, &self.pool).await?;

        if game.finished {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_action: GameReaction::TimedOut,
                    game: game_response,
                    game_id: self.game.nanoid.clone(),
                    user_id: self.user_id,
                    username: self.username.clone(),
                }))),
            });
        }
        // TODO: Figure why/whether we need this code :D
        // messages.push(InternalServerMessage {
        //     destination: MessageDestination::Game(self.game.nanoid.clone()),
        //     message: ServerMessage::Game(TimeoutCheck(game_response)),
        // });
        Ok(messages)
    }
}
