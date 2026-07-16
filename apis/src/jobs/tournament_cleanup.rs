use crate::{
    common::{ServerMessage, ServerResult, TournamentUpdate},
    websocket::{MessageDestination, WsHub},
};
use actix_web::web::Data;
use bytes::Bytes;
use codee::{binary::MsgpackSerdeCodec, Encoder};
use db_lib::{get_conn, models::Tournament, DbPool};
use std::{sync::Arc, time::Duration};

pub fn run(pool: DbPool, hub: Data<Arc<WsHub>>) {
    actix_rt::spawn(async move {
        let mut interval = actix_rt::time::interval(Duration::from_secs(60 * 60 * 24));
        loop {
            interval.tick().await;
            if let Ok(mut conn) = get_conn(&pool).await {
                if let Ok(tournament_ids) = Tournament::delete_old_and_unstarted(&mut conn).await {
                    for tournament_id in tournament_ids {
                        hub.invalidate_tournament_members(&tournament_id);
                        let result = ServerResult::Ok(Box::new(ServerMessage::Tournament(
                            TournamentUpdate::Deleted(tournament_id),
                        )));
                        if let Ok(serialized) = MsgpackSerdeCodec::encode(&result) {
                            hub.dispatch(&MessageDestination::Global, Bytes::from(serialized))
                                .await;
                        }
                    }
                }
            }
        }
    });
}
