use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::GameResponse,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, Schedule},
    DbPool,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use shared_types::GameId;
use uuid::Uuid;

#[derive(Debug)]
pub struct StartHandler {
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl StartHandler {
    pub fn new(
        game: &Game,
        user_id: Uuid,
        username: String,
         pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            user_id,
            username,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        //if let Ok(should_start) = self.data.game_start.should_start(&self.game, self.user_id) {
           if let Some(should_start) = Some(false) { 
            if should_start {
                let game = conn
                    .transaction::<_, anyhow::Error, _>(move |tc| {
                        async move {
                            let started_game = self.game.start(tc).await?;

                            if let Err(e) = Schedule::delete_all_for_game(started_game.id, tc).await
                            {
                                println!(
                                    "Failed to delete schedules for game {}: {}",
                                    started_game.id, e
                                );
                            }

                            Ok(started_game)
                        }
                        .scope_boxed()
                    })
                    .await?;
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Game(GameId(self.game.nanoid.clone())),
                    message: ServerMessage::Game(Box::new(GameUpdate::Reaction(
                        GameActionResponse {
                            game_id: GameId(game.nanoid.to_owned()),
                            game: GameResponse::from_model(&game, &mut conn).await?,
                            game_action: GameReaction::Started,
                            user_id: self.user_id.to_owned(),
                            username: self.username.to_owned(),
                        },
                    ))),
                });
            } else {
                let game_response = GameResponse::from_model(&self.game, &mut conn).await?;
                let game_action_response = GameActionResponse {
                    game_id: GameId(self.game.nanoid.to_owned()),
                    game: game_response,
                    game_action: GameReaction::Ready,
                    user_id: self.user_id.to_owned(),
                    username: self.username.to_owned(),
                };

                messages.push(InternalServerMessage {
                    destination: MessageDestination::Game(GameId(self.game.nanoid.clone())),
                    message: ServerMessage::Game(Box::new(GameUpdate::Reaction(
                        game_action_response.clone(),
                    ))),
                });

                // Also send Ready message to the opponent user (for popup notification)
                let opponent_id = if self.game.white_id == self.user_id {
                    self.game.black_id
                } else {
                    self.game.white_id
                };

                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(opponent_id),
                    message: ServerMessage::Game(Box::new(GameUpdate::Reaction(
                        game_action_response,
                    ))),
                });
            }
        }
        Ok(messages)
    }
}
