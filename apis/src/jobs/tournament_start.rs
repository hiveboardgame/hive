use crate::common::{
    GameActionResponse, GameReaction, GameUpdate, ServerMessage, ServerResult, TournamentUpdate
};
use crate::responses::{GameResponse, TournamentResponse};
use crate::websockets::internal_server_message::{InternalServerMessage, MessageDestination};
use crate::websockets::lobby::Lobby;
use crate::websockets::messages::ClientActorMessage;
use actix::Addr;
use actix_web::web::Data;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use std::time::Duration;

pub fn run(pool: DbPool, lobby: Data<Addr<Lobby>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            println!("Checking for tournaments to be started...");
            if let Ok(mut conn) = get_conn(&pool).await {
                let lobby = lobby.clone();
                let _ = conn
                    .transaction::<_, anyhow::Error, _>(move |tc| {
                        async move {
                            if let Ok(tournament_infos) = Tournament::automatic_start(tc).await {
                                let mut messages = Vec::new();
                                for (tournament, games, deleted_invitations) in tournament_infos {
                                    let tournament_response =
                                        TournamentResponse::from_model(&tournament, tc).await?;

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

                                    messages.push(InternalServerMessage {
                                        destination: MessageDestination::Global,
                                        message: ServerMessage::Tournament(
                                            TournamentUpdate::Started(tournament_response),
                                        ),
                                    });

                                    for game in games {
                                        let game_response =
                                            GameResponse::from_model(&game, tc).await?;

                                        messages.push(InternalServerMessage {
                                            destination: MessageDestination::User(game.white_id),
                                            message: ServerMessage::Game(Box::new(
                                                GameUpdate::Reaction(GameActionResponse {
                                                    game_action: GameReaction::New,
                                                    game: game_response.clone(),
                                                    game_id: game_response.game_id.clone(),
                                                    user_id: game.white_id,
                                                    username: game_response
                                                        .white_player
                                                        .username
                                                        .clone(),
                                                }),
                                            )),
                                        });

                                        messages.push(InternalServerMessage {
                                            destination: MessageDestination::User(game.black_id),
                                            message: ServerMessage::Game(Box::new(
                                                GameUpdate::Reaction(GameActionResponse {
                                                    game_action: GameReaction::New,
                                                    game: game_response.clone(),
                                                    game_id: game_response.game_id.clone(),
                                                    user_id: game.black_id,
                                                    username: game_response.black_player.username,
                                                }),
                                            )),
                                        });
                                    }
                                }
                                for message in messages {
                                    let serialized = serde_json::to_string(&ServerResult::Ok(
                                        Box::new(message.message),
                                    ))
                                    .expect("Failed to serialize a server message");
                                    let cam = ClientActorMessage {
                                        destination: message.destination,
                                        serialized,
                                        from: None,
                                    };
                                    lobby.do_send(cam);
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
