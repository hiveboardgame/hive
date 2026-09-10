use crate::{
    common::GameReaction,
    responses::TournamentPatch,
    websocket::{
        messages::HandlerOutput,
        server_handlers::{
            game::{project_committed_game, GameCommandContext},
            tournaments::public_tournament_patch_message,
        },
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{
    game_command::{self, Command, Outcome},
    get_conn,
    models::{Game, Tournament},
    DbPool,
};
use shared_types::{GameId, TournamentId};
use std::sync::Arc;
use uuid::Uuid;

pub struct BerserkHandler {
    game_id: GameId,
    user_id: Uuid,
    data: Arc<WebsocketData>,
    pool: DbPool,
}

impl BerserkHandler {
    pub fn new(game_id: GameId, user_id: Uuid, data: Arc<WebsocketData>, pool: &DbPool) -> Self {
        Self {
            game_id,
            user_id,
            data,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let game = Game::find_by_game_id(&self.game_id, &mut conn).await?;
        let outcome = game_command::execute(
            game.id,
            Command::Berserk {
                user_id: self.user_id,
            },
            &mut conn,
        )
        .await?;
        let berserk = match &outcome {
            Outcome::Applied {
                game,
                newly_terminal: false,
                ..
            } => game.tournament_id.zip(game.user_color(self.user_id)).map(
                |(tournament_id, color)| {
                    (
                        tournament_id,
                        TournamentPatch::ArenaBerserked {
                            game_id: GameId(game.nanoid.clone()),
                            color,
                        },
                    )
                },
            ),
            _ => None,
        };
        let context = GameCommandContext::websocket_actor(GameReaction::Berserk, self.user_id);
        let mut committed =
            project_committed_game(outcome, context, self.data.as_ref(), &mut conn).await;
        if committed.removed {
            return Err(anyhow::anyhow!("Berserk unexpectedly removed its game"));
        }
        if let Some(reason) = committed.rejected.take() {
            committed.output.request_error = Some(reason.into());
            return Ok(committed.output);
        }
        if let Some((tournament_id, patch)) = berserk {
            match Tournament::find(tournament_id, &mut conn).await {
                Ok(tournament) => committed
                    .output
                    .messages
                    .push(public_tournament_patch_message(
                        TournamentId(tournament.nanoid),
                        patch,
                    )),
                Err(error) => {
                    log::error!("Berserk committed but tournament routing failed: {error}")
                }
            }
        }
        Ok(committed.output)
    }
}
