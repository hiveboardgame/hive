use std::sync::Arc;

use crate::{
    common::{GameActionResponse, GameReaction},
    websocket::{
        messages::{HandlerOutput, Reaction},
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, Schedule},
    DbPool,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use shared_types::GameId;
use uuid::Uuid;

#[derive(Debug)]
pub struct StartHandler {
    pool: DbPool,
    data: Arc<WebsocketData>,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl StartHandler {
    pub fn new(
        game: &Game,
        user_id: Uuid,
        username: String,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            user_id,
            username,
            data: data.clone(),
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let mut reactions = Vec::new();
        if let Ok(should_start) = self.data.game_start.should_start(&self.game, self.user_id) {
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
                let game_response = self.data.get_or_build_response(&game, &mut conn).await?;
                reactions.push(Reaction {
                    game_id: GameId(game.nanoid.to_owned()),
                    white_id: game.white_id,
                    black_id: game.black_id,
                    gar: GameActionResponse {
                        game_id: GameId(game.nanoid.to_owned()),
                        game: (*game_response).clone(),
                        game_action: GameReaction::Started,
                        user_id: self.user_id.to_owned(),
                        username: self.username.to_owned(),
                    },
                });
            } else {
                let game_response = self
                    .data
                    .get_or_build_response(&self.game, &mut conn)
                    .await?;
                reactions.push(Reaction {
                    game_id: GameId(self.game.nanoid.clone()),
                    white_id: self.game.white_id,
                    black_id: self.game.black_id,
                    gar: GameActionResponse {
                        game_id: GameId(self.game.nanoid.to_owned()),
                        game: (*game_response).clone(),
                        game_action: GameReaction::Ready,
                        user_id: self.user_id.to_owned(),
                        username: self.username.to_owned(),
                    },
                });
            }
        }
        Ok(HandlerOutput {
            messages: Vec::new(),
            reactions,
            finalize_games: Vec::new(),
            subscriptions: Vec::new(),
        })
    }
}
