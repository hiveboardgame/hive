use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage, TournamentUpdate},
    responses::GameResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use hive_lib::GameControl;
use shared_types::{TournamentGameResult, TournamentId};
use uuid::Uuid;

pub struct AbandonHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    username: String,
    pool: DbPool,
}

impl AbandonHandler {
    pub async fn new(
        tournament_id: TournamentId,
        user_id: Uuid,
        username: String,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            username,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        // WARN: This currently only works for one round tournaments. For all other tournaments we
        // need to also remove the player from players somehow. So that no future matches are
        // scheduled against them.
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        let abandoned = conn
            .transaction::<_, DbError, _>(move |tc| {
                async move {
                    let mut abandoned = Vec::new();
                    let tournament =
                        Tournament::find_by_tournament_id(&self.tournament_id, tc).await?;
                    for game in tournament.games(tc).await?.iter() {
                        if let Some(color) = game.user_color(self.user_id) {
                            abandoned.push(game.resign(&GameControl::Resign(color), tc).await?);
                        }
                    }
                    Ok(abandoned)
                }
                .scope_boxed()
            })
            .await?;

        let game_responses = GameResponse::from_games_batch(abandoned, &mut conn).await?;
        for game_response in game_responses {
            let color = match game_response.tournament_game_result {
                TournamentGameResult::Winner(color) => color.opposite_color(),
                _ => unreachable!("Tournament game should have a winner when player abandons"),
            };
            let game_control = GameControl::Resign(color);
            messages.push(InternalServerMessage {
                destination: MessageDestination::Game(game_response.game_id.clone()),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_id: game_response.game_id.clone(),
                    game: game_response.clone(),
                    game_action: GameReaction::Control(game_control),
                    user_id: self.user_id.to_owned(),
                    username: self.username.to_owned(),
                }))),
            });
        }

        let tournament_response = self.tournament_id.clone();

        messages.push(InternalServerMessage {
            destination: MessageDestination::Tournament(tournament_response.clone()),
            message: ServerMessage::Tournament(TournamentUpdate::Modified(tournament_response)),
        });
        Ok(messages)
    }
}
