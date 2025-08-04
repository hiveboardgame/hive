use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage, TournamentUpdate},
    responses::GameResponse,
    websocket::busybee::Busybee,
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

        // Get all players before starting the tournament
        let players = tournament.players(&mut conn).await?;

        let (tournament, games, deleted_invitations) = conn
            .transaction::<_, DbError, _>(move |tc| {
                async move { tournament.start_by_organizer(&self.user_id, tc).await }.scope_boxed()
            })
            .await?;

        // Send busybee messages to all tournament participants
        for player in players {
            let msg = format!(
                "[Tournament Started](<https://hivegame.com/tournament/{}>) - {} has begun! Your games are ready.",
                tournament.nanoid,
                tournament.name
            );

            if let Err(e) = Busybee::msg(player.id, msg).await {
                println!("Failed to send Busybee message to {}: {e}", player.username);
            }
        }

        for uuid in deleted_invitations {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(uuid),
                message: ServerMessage::Tournament(TournamentUpdate::Uninvited(TournamentId(
                    tournament.nanoid.clone(),
                ))),
            });
        }

        messages.push(InternalServerMessage {
            destination: MessageDestination::Tournament(self.tournament_id.clone()),
            message: ServerMessage::Tournament(TournamentUpdate::Started(TournamentId(
                tournament.nanoid.clone(),
            ))),
        });

        let game_responses = GameResponse::from_games_batch(games, &mut conn).await?;
        for game in game_responses {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(game.white_player.uid),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_action: GameReaction::New,
                    game: game.clone(),
                    game_id: game.game_id.clone(),
                    user_id: game.white_player.uid,
                    username: game.white_player.username.clone(),
                }))),
            });

            messages.push(InternalServerMessage {
                destination: MessageDestination::User(game.black_player.uid),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_action: GameReaction::New,
                    game: game.clone(),
                    game_id: game.game_id.clone(),
                    user_id: game.black_player.uid,
                    username: game.black_player.username,
                }))),
            });
        }
        Ok(messages)
    }
}
