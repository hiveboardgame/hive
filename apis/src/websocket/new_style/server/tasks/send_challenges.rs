use crate::common::{ChallengeUpdate, ServerMessage};
use crate::responses::ChallengeResponse;
use crate::websocket::new_style::server::{ServerData, TabData};
use db_lib::get_conn;
use db_lib::models::Challenge;

pub async fn send_challenges(client: &TabData, server: &ServerData) {
    // Send challenges on join
    let mut conn = match get_conn(client.pool()).await {
        Ok(conn) => conn,
        Err(e) => {
            println!("Failed to get connection for send_challenges {e}");
            return;
        }
    };

    if let Some(account) = client.account() {
        let user_id = account.id;
        let mut responses = Vec::new();

        if let Ok(challenges) = Challenge::get_public_exclude_user(user_id, &mut conn).await {
            for challenge in challenges {
                if let Ok(response) = ChallengeResponse::from_model(&challenge, &mut conn).await {
                    responses.push(response);
                }
            }
        }

        if let Ok(challenges) = Challenge::get_own(user_id, &mut conn).await {
            for challenge in challenges {
                if let Ok(response) = ChallengeResponse::from_model(&challenge, &mut conn).await {
                    responses.push(response);
                }
            }
        }

        if let Ok(challenges) = Challenge::direct_challenges(user_id, &mut conn).await {
            for challenge in challenges {
                if let Ok(response) = ChallengeResponse::from_model(&challenge, &mut conn).await {
                    responses.push(response);
                }
            }
        }

        let message = ServerMessage::Challenge(ChallengeUpdate::Challenges(responses));
        client.send(message, server);
    } else {
        let mut responses = Vec::new();

        if let Ok(challenges) = Challenge::get_public(&mut conn).await {
            for challenge in challenges {
                if let Ok(response) = ChallengeResponse::from_model(&challenge, &mut conn).await {
                    responses.push(response);
                }
            }
        }

        let message = ServerMessage::Challenge(ChallengeUpdate::Challenges(responses));
        client.send(message, server);
    }
}
