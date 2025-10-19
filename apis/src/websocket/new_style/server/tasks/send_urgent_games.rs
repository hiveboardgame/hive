use std::sync::Arc;

use db_lib::get_conn;
use db_lib::models::User;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;

use crate::common::{GameUpdate, ServerMessage};
use crate::responses::GameResponse;
use crate::websocket::new_style::server::{ServerData, TabData};

pub async fn send_urgent_games(client: TabData, server_data: Arc<ServerData>) {
    // Send games which require input from the user
    let Some(user_id) = client.account().map(|account| account.id) else {
        return;
    };

    let mut conn = match get_conn(client.pool()).await {
        Ok(conn) => conn,
        Err(e) => {
            println!("Failed to get connection for urgent games {user_id}: {e}");
            return;
        }
    };

    let user = match User::find_by_uuid(&user_id, &mut conn).await {
        Ok(user) => user,
        Err(e) => {
            println!("Failed to get user for urgent games {user_id}: {e}");
            return;
        }
    };

    let game_ids = match user.get_urgent_nanoids(&mut conn).await {
        Ok(ids) => ids,
        Err(e) => {
            println!("Failed to get urgent game_ids for user {user_id}: {e}");
            Vec::new()
        }
    };

    if game_ids.is_empty() {
        return;
    }

    let games = conn
        .transaction::<_, anyhow::Error, _>(move |tc| {
            async move {
                let mut games = Vec::new();
                for game_id in game_ids {
                    if let Ok(game) = GameResponse::new_from_game_id(&game_id, tc).await {
                        if !game.finished {
                            games.push(game);
                        }
                    }
                }
                Ok(games)
            }
            .scope_boxed()
        })
        .await;

    let games = match games {
        Ok(games) => games,
        Err(e) => {
            println!("Failed to get urgent games for user {user_id}: {e}");
            return;
        }
    };

    let messages = vec![ServerMessage::Game(Box::new(GameUpdate::Urgent(games)))];
    for message in messages {
        client.send(message, &server_data).await;
    }
}
