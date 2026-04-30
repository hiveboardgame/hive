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
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use shared_types::TournamentId;
use std::sync::Arc;
use std::time::Duration;

pub fn run(pool: DbPool, hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok(mut conn) = get_conn(&pool).await {
                let hub = hub.get_ref().clone();
                let _ = conn
                    .transaction::<_, anyhow::Error, _>(move |tc| {
                        async move {
                            if let Ok(tournament_infos) = Tournament::automatic_start(tc).await {
                                let mut messages = Vec::new();
                                for (tournament, games, deleted_invitations) in tournament_infos {
                                    let tournament_response =
                                        TournamentId(tournament.nanoid.clone());

                                    for uuid in deleted_invitations {
                                        messages.push(InternalServerMessage {
                                            destination: MessageDestination::User(uuid),
                                            message: ServerMessage::Tournament(
                                                TournamentUpdate::Uninvited(
                                                    tournament_response.clone(),
                                                ),
                                            ),
                                        });
                                    }

                                    let players = tournament.players(tc).await?;
                                    for player in players {
                                        messages.push(InternalServerMessage {
                                            destination: MessageDestination::User(player.id),
                                            message: ServerMessage::Tournament(
                                                TournamentUpdate::Started(
                                                    tournament_response.clone(),
                                                ),
                                            ),
                                        });
                                    }

                                    let game_responses =
                                        GameResponse::from_games_batch(games, tc).await?;
                                    for game in game_responses {
                                        messages.push(InternalServerMessage {
                                            destination: MessageDestination::User(
                                                game.white_player.uid,
                                            ),
                                            message: ServerMessage::Game(Box::new(
                                                GameUpdate::Reaction(GameActionResponse {
                                                    game_action: GameReaction::New,
                                                    game: game.clone(),
                                                    game_id: game.game_id.clone(),
                                                    user_id: game.white_player.uid,
                                                    username: game.white_player.username.clone(),
                                                }),
                                            )),
                                        });

                                        messages.push(InternalServerMessage {
                                            destination: MessageDestination::User(
                                                game.black_player.uid,
                                            ),
                                            message: ServerMessage::Game(Box::new(
                                                GameUpdate::Reaction(GameActionResponse {
                                                    game_action: GameReaction::New,
                                                    game: game.clone(),
                                                    game_id: game.game_id.clone(),
                                                    user_id: game.black_player.uid,
                                                    username: game.black_player.username,
                                                }),
                                            )),
                                        });
                                    }
                                }
                                for message in messages {
                                    let serialized = ServerResult::Ok(Box::new(message.message));
                                    if let Ok(serialized) = MsgpackSerdeCodec::encode(&serialized) {
                                        hub.dispatch(
                                            &message.destination,
                                            Bytes::from(serialized),
                                            None,
                                        )
                                        .await;
                                    };
                                }
                            }
                            Ok(())
                        }
                        .scope_boxed()
                    })
                    .await;
            }
        }
    });
}
