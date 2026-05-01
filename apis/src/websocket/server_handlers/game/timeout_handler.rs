use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        WebsocketData,
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use shared_types::GameId;
use std::sync::Arc;
use uuid::Uuid;

pub struct TimeoutHandler {
    game: Game,
    username: String,
    user_id: Uuid,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
    pool: DbPool,
}

impl TimeoutHandler {
    pub fn new(
        game: &Game,
        username: &str,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            username: username.to_owned(),
            user_id,
            data,
            hub,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();

        if self.game.finished {
            let game_response = self.data.get_or_build_response(&self.game, &mut conn).await?;
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_action: GameReaction::TimedOut,
                    game: (*game_response).clone(),
                    game_id: GameId(self.game.nanoid.clone()),
                    user_id: self.user_id,
                    username: self.username.clone(),
                }))),
            });
            self.hub.on_game_finished(
                &GameId(self.game.nanoid.clone()),
                self.game.white_id,
                self.game.black_id,
            );
        }

        Ok(messages)
    }
}
