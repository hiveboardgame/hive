use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::{InternalServerMessage, MessageDestination, WsHub},
};
use actix_web::web::Data;
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
                        for update in [
                            TournamentUpdate::Deleted(tournament_id.clone()),
                            TournamentUpdate::CatalogChanged(tournament_id),
                        ] {
                            let message = InternalServerMessage {
                                destination: MessageDestination::Global,
                                message: ServerMessage::Tournament(update),
                            };
                            let _ = hub.dispatch_message(message).await;
                        }
                    }
                }
            }
        }
    });
}
