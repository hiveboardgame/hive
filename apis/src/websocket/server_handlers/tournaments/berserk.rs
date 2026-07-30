use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::GameResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::{bail, Result};
use db_lib::{get_conn, models::Game, DbPool};
use diesel_async::AsyncConnection;
use shared_types::GameId;
use uuid::Uuid;

/// Declaring berserk on one's own game: half the clock and none of the
/// increment, in exchange for the arena's scoring bonus.
pub struct BerserkHandler {
    game_id: GameId,
    user_id: Uuid,
    username: String,
    pool: DbPool,
}

impl BerserkHandler {
    pub fn new(game_id: GameId, user_id: Uuid, username: String, pool: &DbPool) -> Self {
        Self {
            game_id,
            user_id,
            username,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let game = Game::find_by_game_id(&self.game_id, &mut conn).await?;

        // Only the two players can berserk, and only their own side of it.
        let Some(color) = game.user_color(self.user_id) else {
            bail!("only a player of this game can berserk it");
        };

        let updated = conn
            .transaction::<_, anyhow::Error, _>(async move |tc| Ok(game.berserk(color, tc).await?))
            .await?;

        let response = GameResponse::from_model(&updated, &mut conn).await?;
        // Both sides need it: the opponent's clock display depends on whether
        // this player berserked.
        let recipients = [updated.white_id, updated.black_id];
        Ok(recipients
            .into_iter()
            .map(|recipient| InternalServerMessage {
                destination: MessageDestination::User(recipient),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_action: GameReaction::Berserk,
                    game: response.clone(),
                    game_id: self.game_id.clone(),
                    user_id: self.user_id,
                    username: self.username.clone(),
                }))),
            })
            .collect())
    }
}
