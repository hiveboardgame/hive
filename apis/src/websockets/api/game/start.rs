use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::{GameResponse, UserResponse},
    websockets::{
        internal_server_message::{InternalServerMessage, MessageDestination},
        tournament_game_start::TournamentGameStart,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use shared_types::GameId;
use uuid::Uuid;

#[derive(Debug)]
pub struct StartHandler {
    pool: DbPool,
    game_start: actix_web::web::Data<TournamentGameStart>,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl StartHandler {
    pub fn new(
        game: &Game,
        user_id: Uuid,
        username: String,
        game_start: actix_web::web::Data<TournamentGameStart>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            user_id,
            username,
            game_start,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        if let Ok(should_start) = self.game_start.should_start(&self.game, self.user_id) {
            if should_start {
                let game = conn
                    .transaction::<_, anyhow::Error, _>(move |tc| {
                        async move { Ok(self.game.start(tc).await?) }.scope_boxed()
                    })
                    .await?;
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Game(self.game.nanoid.clone()),
                    message: ServerMessage::Ready(
                        UserResponse::from_uuid(&self.user_id, &mut conn).await?,
                    ),
                });
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Game(game.nanoid.clone()),
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
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Game(self.game.nanoid.clone()),
                    message: ServerMessage::Ready(
                        UserResponse::from_uuid(&self.user_id, &mut conn).await?,
                    ),
                });
            }
        }
        Ok(messages)
    }
}
