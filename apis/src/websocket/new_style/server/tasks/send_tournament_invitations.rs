use std::sync::Arc;

use crate::common::{ServerMessage, TournamentUpdate};
use crate::responses::TournamentResponse;
use crate::websocket::new_style::server::{ServerData, TabData};
use db_lib::get_conn;
use db_lib::models::TournamentInvitation;

pub async fn send_tournament_invitations(client: TabData, server: Arc<ServerData>) {
    let mut conn = match get_conn(client.pool()).await {
        Ok(conn) => conn,
        Err(_) => {
            println!("Failed to get connection for tournament invitations");
            return;
        }
    };

    if let Some(account) = client.account() {
        let user_id = account.id;

        if let Ok(invitations) = TournamentInvitation::find_by_user(&user_id, &mut conn).await {
            for invitation in invitations {
                if let Ok(response) =
                    TournamentResponse::from_uuid(&invitation.tournament_id, &mut conn).await
                {
                    client
                        .send(
                            ServerMessage::Tournament(TournamentUpdate::Invited(
                                response.tournament_id.clone(),
                            )),
                            &server,
                        );
                }
            }
        }
    }
}
