use crate::{
    common::{
        GameReaction, {GameActionResponse, GameUpdate, ServerMessage},
    },
    responses::GameResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use uuid::Uuid;

pub struct TimeoutHandler {
    game: Game,
    username: String,
    user_id: Uuid,
    pool: DbPool,
}

impl TimeoutHandler {
    pub fn new(game: &Game, username: &str, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            game: game.to_owned(),
            username: username.to_owned(),
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();

        let game = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move {
                    // find_by_nanoid automatically times the game out if needed
                    Ok(Game::find_by_nanoid(&self.game.nanoid, tc).await?)
                }
                .scope_boxed()
            })
            .await?;

        let game_response = GameResponse::new_from_db(&game, &mut conn).await?;
        if game.finished {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_action: GameReaction::TimedOut,
                    game: game_response,
                    game_id: game.nanoid,
                    user_id: self.user_id,
                    username: self.username.clone(),
                }))),
            });
        }

        Ok(messages)
    }
}
