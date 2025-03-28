use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage, TournamentUpdate},
    responses::{GameResponse, TournamentResponse},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{db_error::DbError, get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use shared_types::TournamentId;
use uuid::Uuid;

pub struct StartHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
}

impl StartHandler {
    pub async fn new(tournament_id: TournamentId, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let (tournament, games, deleted_invitations) = conn
            .transaction::<_, DbError, _>(move |tc| {
                async move { tournament.start_by_organizer(&self.user_id, tc).await }.scope_boxed()
            })
            .await?;
        let tournament_response = TournamentResponse::from_model(&tournament, &mut conn).await?;

        for uuid in deleted_invitations {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(uuid),
                message: ServerMessage::Tournament(TournamentUpdate::Uninvited(
                    tournament_response.clone(),
                )),
            });
        }

        let players = tournament.players(&mut conn).await?;

        for player in players {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(player.id),
                message: ServerMessage::Tournament(TournamentUpdate::Started(
                    tournament_response.clone(),
                )),
            });
        }

        for game in games {
            let game_response = GameResponse::from_model(&game, &mut conn).await?;

            messages.push(InternalServerMessage {
                destination: MessageDestination::User(game.white_id),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_action: GameReaction::New,
                    game: game_response.clone(),
                    game_id: game_response.game_id.clone(),
                    user_id: game.white_id,
                    username: game_response.white_player.username.clone(),
                }))),
            });

            messages.push(InternalServerMessage {
                destination: MessageDestination::User(game.black_id),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_action: GameReaction::New,
                    game: game_response.clone(),
                    game_id: game_response.game_id.clone(),
                    user_id: game.black_id,
                    username: game_response.black_player.username,
                }))),
            });
        }
        Ok(messages)
    }
}
