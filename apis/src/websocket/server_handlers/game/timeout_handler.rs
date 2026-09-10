use super::tournament_progression::settle_deadline;
use crate::websocket::{messages::HandlerOutput, WebsocketData};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use std::sync::Arc;
use uuid::Uuid;

pub struct TimeoutHandler {
    game_id: Uuid,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl TimeoutHandler {
    pub fn new(game: &Game, data: Arc<WebsocketData>, pool: &DbPool) -> Self {
        Self {
            game_id: game.id,
            data,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let projected = settle_deadline(self.game_id, self.data.as_ref(), &mut conn).await?;
        Ok(projected.output)
    }
}
