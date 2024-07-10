use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage, TournamentUpdate},
    responses::{GameResponse, TournamentResponse},
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use hive_lib::GameControl;
use shared_types::{GameId, TournamentId};
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
        let (tournament, abandoned) = conn
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
                    Ok((tournament, abandoned))
                }
                .scope_boxed()
            })
            .await?;

        for game in abandoned {
            let game_response = GameResponse::from_model(&game, &mut conn).await?;
            let color = game
                .user_color(self.user_id)
                .expect("User who aborted game to be player");
            let game_control = GameControl::Resign(color);
            messages.push(InternalServerMessage {
                destination: MessageDestination::Game(GameId(game.nanoid.clone())),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_id: GameId(game.nanoid.to_owned()),
                    game: game_response.clone(),
                    game_action: GameReaction::Control(game_control),
                    user_id: self.user_id.to_owned(),
                    username: self.username.to_owned(),
                }))),
            });
        }

        let tournament_response = TournamentResponse::from_model(&tournament, &mut conn).await?;

        messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Modified(tournament_response)),
        });
        Ok(messages)
    }
}
