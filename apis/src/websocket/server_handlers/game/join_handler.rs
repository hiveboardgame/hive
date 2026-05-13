use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    websocket::{
        messages::{
            GameSubscription,
            HandlerOutput,
            InternalServerMessage,
            MessageDestination,
            SocketTx,
        },
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use shared_types::GameId;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub struct JoinHandler {
    pool: DbPool,
    received_from: SocketTx,
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
        received_from: SocketTx,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            received_from,
            data,
            game: game.to_owned(),
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        let game_id = GameId(self.game.nanoid.clone());
        let mut subscriptions = vec![GameSubscription::Fanout(game_id.clone())];
        if !self.game.finished {
            subscriptions.push(GameSubscription::Heartbeat(game_id.clone()));
        }
        messages.push(InternalServerMessage {
            destination: MessageDestination::Game(game_id.clone()),
            message: ServerMessage::Join(self.user_id),
        });
        let game_response = self
            .data
            .get_or_build_response(&self.game, &mut conn)
            .await?;
        messages.push(InternalServerMessage {
            destination: MessageDestination::Direct(self.received_from.clone()),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_id: GameId(self.game.nanoid.to_owned()),
                game: (*game_response).clone(),
                game_action: GameReaction::Join,
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            }))),
        });
        Ok(HandlerOutput {
            messages,
            subscriptions,
            reactions: Vec::new(),
            finalize_games: Vec::new(),
        })
    }
}
