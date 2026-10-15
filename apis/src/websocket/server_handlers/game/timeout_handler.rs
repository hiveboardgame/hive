use super::projection::settle_deadline;
use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use hive_lib::GameStatus;
use shared_types::{Conclusion, GameId};
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

pub struct TimeoutHandler {
    game_id: Uuid,
    received_from: SocketTx,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl TimeoutHandler {
    pub fn new(
        game: &Game,
        received_from: SocketTx,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game_id: game.id,
            received_from,
            data,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let mut projected = settle_deadline(self.game_id, self.data.as_ref(), &mut conn).await?;
        if projected.game.finished && projected.output.finalize_games.is_empty() {
            let game = self
                .data
                .get_or_build_response(&projected.game, &mut conn)
                .await?;
            let reaction = if projected.game.conclusion == Conclusion::Timeout.to_string() {
                GameReaction::TimedOut
            } else if matches!(
                GameStatus::from_str(&projected.game.game_status),
                Ok(GameStatus::Adjudicated)
            ) {
                GameReaction::Adjudicated
            } else {
                GameReaction::Finished
            };
            projected.output.messages.push(InternalServerMessage {
                destination: MessageDestination::Direct(self.received_from.clone()),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_id: GameId(projected.game.nanoid.clone()),
                    game: (*game).clone(),
                    game_action: reaction,
                    user_id: projected.game.current_player_id,
                    username: String::new(),
                }))),
            });
        }
        Ok(projected.output)
    }
}
