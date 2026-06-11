use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    websocket::{
        messages::{GameFinalize, HandlerOutput, InternalServerMessage, MessageDestination},
        WebsocketData,
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
    pool: DbPool,
}

impl TimeoutHandler {
    pub fn new(
        game: &Game,
        username: &str,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            username: username.to_owned(),
            user_id,
            data,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        let mut finalize_games = Vec::new();

        if self.game.finished {
            let game_response = self
                .data
                .get_or_build_response(&self.game, &mut conn)
                .await?;
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
            crate::notifications::notify_game_ended(
                &self.game,
                crate::notifications::GameEndReason::Timeout,
                &mut conn,
            )
            .await?;
            let finalize = GameFinalize {
                game_id: GameId(self.game.nanoid.clone()),
                white_id: self.game.white_id,
                black_id: self.game.black_id,
            };
            messages.extend(finalize.own_game_removed_messages());
            finalize_games.push(finalize);
        }

        Ok(HandlerOutput {
            messages,
            reactions: Vec::new(),
            finalize_games,
            subscriptions: Vec::new(),
        })
    }
}
