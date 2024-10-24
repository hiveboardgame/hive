use crate::{
    common::ServerMessage,
    responses::{GameResponse, GamesSearchResponse},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use shared_types::{BatchInfo, GamesQueryOptions};
use uuid::Uuid;

pub struct GamesSearchHandler {
    options: GamesQueryOptions,
    user_id: Uuid,
    pool: DbPool,
}

impl GamesSearchHandler {
    pub async fn new(user_id: Uuid, options: GamesQueryOptions, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            user_id,
            options,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let options = self.options.clone();
        let mut conn = get_conn(&self.pool).await?;
        let games = Game::get_rows_from_options(&options, &mut conn).await?;
        let mut game_responses = Vec::new();
        for game in &games {
            let game_response = GameResponse::from_model(game, &mut conn).await?;
            game_responses.push(game_response);
        }
        let batch = games.last().map(|game| BatchInfo {
            id: game.id,
            timestamp: game.updated_at,
        });
        let response = GamesSearchResponse {
            results: game_responses,
            batch,
            ctx_to_update: self.options.ctx_to_update.clone(),
            more_rows: options.batch_size.map_or(false, |b| b == games.len()),
            first_batch: options.current_batch.is_none(),
        };
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::GamesSearch(response),
        }])
    }
}
