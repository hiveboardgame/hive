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
use diesel_async::AsyncConnection;
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
        let admitted = async {
            if !self
                .data
                .game_start
                .should_start(&self.game, self.user_id)?
            {
                return Ok::<Option<Game>, anyhow::Error>(None);
            }
            let game = conn
                .transaction::<_, anyhow::Error, _>(async move |tc| {
                    let game = self.game.start(tc).await?;

                    if let Err(e) = Schedule::delete_all_for_game(game.id, tc).await {
                        println!("Failed to delete schedules for game {}: {}", game.id, e);
                    }

                    Ok(game)
                })
                .await?;
            Ok(Some(game))
        };
        let started_game = self
            .data
            .realtime_gate
            .with_realtime_admission(self.game.requires_realtime_admission(), admitted)
            .await?;

        let (game, game_action) = match started_game.as_ref() {
            Some(game) => (game, GameReaction::Started),
            None => (&self.game, GameReaction::Ready),
        };
        let game_response = self.data.get_or_build_response(game, &mut conn).await?;
        Ok(HandlerOutput {
            messages: Vec::new(),
            reactions: vec![Reaction {
                game_id: GameId(game.nanoid.to_owned()),
                white_id: game.white_id,
                black_id: game.black_id,
                gar: GameActionResponse {
                    game_id: GameId(game.nanoid.to_owned()),
                    game: (*game_response).clone(),
                    game_action,
                    user_id: self.user_id.to_owned(),
                    username: self.username.to_owned(),
                },
            }],
            finalize_games: Vec::new(),
        })
    }
}
