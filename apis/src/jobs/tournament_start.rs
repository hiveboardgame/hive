use crate::{
    common::{
        GameActionResponse,
        GameReaction,
        GameUpdate,
        ServerMessage,
        ServerResult,
        TournamentUpdate,
    },
    responses::GameResponse,
    websocket::{InternalServerMessage, MessageDestination, WsHub},
};
use actix_web::web::Data;
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{get_conn, models::Tournament, DbPool};
use shared_types::TournamentId;
use std::{sync::Arc, time::Duration};

pub fn run(pool: DbPool, hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let Ok(mut conn) = get_conn(&pool).await else {
                continue;
            };
            let hub = hub.get_ref().clone();

            // Deliberately not wrapped in one transaction: `automatic_start`
            // opens its own per tournament, so a tournament that cannot start
            // is logged and skipped instead of stopping every other scheduled
            // tournament on the site.
            let tournament_infos = match Tournament::automatic_start(&mut conn).await {
                Ok(tournament_infos) => tournament_infos,
                Err(error) => {
                    tracing::error!(%error, "automatic tournament start failed");
                    continue;
                }
            };

            let mut messages = Vec::new();
            for (tournament, games, deleted_invitations) in tournament_infos {
                let tournament_response = TournamentId(tournament.nanoid.clone());

                for uuid in deleted_invitations {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(uuid),
                        message: ServerMessage::Tournament(TournamentUpdate::Uninvited(
                            tournament_response.clone(),
                        )),
                    });
                }

                // Announced before the per-player messages, so a failure to load
                // the entrants below still leaves the arena advertised.
                if tournament.mode().is_ok_and(|mode| mode.is_arena()) {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::Global,
                        message: ServerMessage::Tournament(TournamentUpdate::ArenaStarted(
                            tournament_response.clone(),
                        )),
                    });
                }

                let Ok(players) = tournament.players(&mut conn).await else {
                    tracing::error!(
                        tournament = %tournament.nanoid,
                        "could not load players of a tournament that just started",
                    );
                    continue;
                };
                for player in players {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(player.id),
                        message: ServerMessage::Tournament(TournamentUpdate::Started(
                            tournament_response.clone(),
                        )),
                    });
                }

                let Ok(game_responses) = GameResponse::from_games_batch(games, &mut conn).await
                else {
                    tracing::error!(
                        tournament = %tournament.nanoid,
                        "could not build responses for a started tournament's games",
                    );
                    continue;
                };
                for game in game_responses {
                    for player in [&game.white_player, &game.black_player] {
                        messages.push(InternalServerMessage {
                            destination: MessageDestination::User(player.uid),
                            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(
                                GameActionResponse {
                                    game_action: GameReaction::New,
                                    game: game.clone(),
                                    game_id: game.game_id.clone(),
                                    user_id: player.uid,
                                    username: player.username.clone(),
                                },
                            ))),
                        });
                    }
                }
            }

            for message in messages {
                let serialized = ServerResult::Ok(Box::new(message.message));
                if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
                    hub.dispatch(&message.destination, Bytes::from(serialized))
                        .await;
                }
            }
        }
    });
}
