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
    websocket::{InternalServerMessage, MessageDestination, RealtimeDisabled, WsHub},
};
use actix_web::web::Data;
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{get_conn, models::Tournament, DbConn, DbPool};
use diesel_async::AsyncConnection;
use shared_types::{TimeMode, TournamentId};
use std::{sync::Arc, time::Duration};

pub fn run(pool: DbPool, hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            let hub = hub.get_ref().clone();

            for time_mode in [TimeMode::Correspondence, TimeMode::RealTime] {
                let result = async {
                    let mut conn = get_conn(&pool).await?;
                    hub.data
                        .realtime_gate
                        .with_realtime_admission(
                            time_mode == TimeMode::RealTime,
                            process_batch(time_mode, &mut conn),
                        )
                        .await
                }
                .await;
                match result {
                    Ok(messages) => dispatch_messages(messages, &hub).await,
                    Err(error) if error.downcast_ref::<RealtimeDisabled>().is_some() => {}
                    Err(error) => {
                        log::error!("automatic {time_mode} tournament start failed: {error}")
                    }
                }
            }
        }
    });
}

async fn process_batch(
    time_mode: TimeMode,
    conn: &mut DbConn<'_>,
) -> anyhow::Result<Vec<InternalServerMessage>> {
    conn.transaction::<_, anyhow::Error, _>(async move |tc| {
        let tournament_infos = Tournament::automatic_start(time_mode, tc).await?;
        build_messages(tournament_infos, tc).await
    })
    .await
}

async fn dispatch_messages(messages: Vec<InternalServerMessage>, hub: &Arc<WsHub>) {
    for message in messages {
        let result = ServerResult::Ok(Box::new(message.message));
        if let Ok(serialized) = MsgpackSerdeCodec::encode(&result) {
            hub.dispatch(&message.destination, Bytes::from(serialized))
                .await;
        }
    }
}

async fn build_messages(
    tournament_infos: Vec<(Tournament, Vec<db_lib::models::Game>, Vec<uuid::Uuid>)>,
    conn: &mut DbConn<'_>,
) -> anyhow::Result<Vec<InternalServerMessage>> {
    let mut messages = Vec::new();
    for (tournament, games, deleted_invitations) in tournament_infos {
        let tournament_id = TournamentId(tournament.nanoid.clone());
        for uuid in deleted_invitations {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(uuid),
                message: ServerMessage::Tournament(TournamentUpdate::Uninvited(
                    tournament_id.clone(),
                )),
            });
        }
        for player in tournament.players(conn).await? {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(player.id),
                message: ServerMessage::Tournament(TournamentUpdate::Started(
                    tournament_id.clone(),
                )),
            });
        }
        for game in GameResponse::from_games_batch(games, conn).await? {
            for (user_id, username) in [
                (game.white_player.uid, game.white_player.username.clone()),
                (game.black_player.uid, game.black_player.username.clone()),
            ] {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(user_id),
                    message: ServerMessage::Game(Box::new(GameUpdate::Reaction(
                        GameActionResponse {
                            game_action: GameReaction::New,
                            game: game.clone(),
                            game_id: game.game_id.clone(),
                            user_id,
                            username,
                        },
                    ))),
                });
            }
        }
    }
    Ok(messages)
}
