//! Server functions for chat read receipts and unread counts.

use leptos::prelude::*;

#[cfg(feature = "ssr")]
const VALID_CHANNEL_TYPES: [&str; 5] = [
    shared_types::CHANNEL_TYPE_GAME_PLAYERS,
    shared_types::CHANNEL_TYPE_GAME_SPECTATORS,
    shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY,
    shared_types::CHANNEL_TYPE_DIRECT,
    shared_types::CHANNEL_TYPE_GLOBAL,
];

#[cfg(feature = "ssr")]
fn is_valid_channel_type(t: &str) -> bool {
    VALID_CHANNEL_TYPES.contains(&t)
}

#[server]
pub async fn mark_chat_read(channel_type: String, channel_id: String) -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use chrono::Utc;
    use db_lib::{get_conn, helpers::{can_user_access_chat_channel, upsert_chat_read_receipt}};

    // Trim in case client sends trailing/leading spaces; other endpoints use exact constants.
    let channel_type = channel_type.trim();
    if !is_valid_channel_type(channel_type) {
        return Err(ServerFnError::new("Invalid channel_type"));
    }
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(ServerFnError::new)?;

    // Verify user can access this channel before creating a read receipt
    let allowed = can_user_access_chat_channel(&mut conn, user_id, channel_type, &channel_id)
        .await
        .map_err(ServerFnError::new)?;
    if !allowed {
        return Err(ServerFnError::new("Access denied"));
    }

    upsert_chat_read_receipt(&mut conn, user_id, channel_type, &channel_id, Utc::now())
        .await
        .map_err(ServerFnError::new)?;
    Ok(())
}

/// Returns (channel_type, channel_id, unread_count) for channels where the user has a receipt and count > 0.
#[server]
pub async fn get_chat_unread_counts(
) -> Result<Vec<(String, String, i64)>, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::{get_conn, helpers::get_unread_counts_for_user};

    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(ServerFnError::new)?;
    get_unread_counts_for_user(&mut conn, user_id)
        .await
        .map_err(ServerFnError::new)
}

/// Response for list of conversations for the Messages hub.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MyConversations {
    pub dms: Vec<DmConversation>,
    pub tournaments: Vec<TournamentChannel>,
    pub games: Vec<GameChannel>,
    pub has_global: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DmConversation {
    pub other_user_id: uuid::Uuid,
    pub username: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TournamentChannel {
    pub nanoid: String,
    pub name: String,
    /// True if the current user is a participant (player or organizer) and can send messages.
    pub is_participant: bool,
    /// True if the user has muted this tournament's lobby chat (no live push, no unread).
    pub muted: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameChannel {
    pub channel_type: String,
    pub channel_id: String,
    pub label: String,
    pub white_id: uuid::Uuid,
    pub black_id: uuid::Uuid,
    /// True when the game is finished; client fetches both channels and merges for display.
    pub finished: bool,
}

#[server]
pub async fn get_my_chat_conversations() -> Result<MyConversations, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::{
        get_conn,
        helpers::{
            get_dm_conversations_for_user, get_game_channels_for_user,
            get_muted_tournament_nanoids, get_tournament_lobby_channels_for_user,
            global_channel_has_messages, is_tournament_participant,
        },
    };

    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(ServerFnError::new)?;

    let dms = get_dm_conversations_for_user(&mut conn, user_id)
        .await
        .map_err(ServerFnError::new)?
        .into_iter()
        .map(|(id, username)| DmConversation {
            other_user_id: id,
            username,
        })
        .collect();

    let tournament_rows = get_tournament_lobby_channels_for_user(&mut conn, user_id)
        .await
        .map_err(ServerFnError::new)?;
    let muted_nanoids = get_muted_tournament_nanoids(&mut conn, user_id).await.unwrap_or_default();
    let tournaments: Vec<TournamentChannel> = {
        let mut out = Vec::with_capacity(tournament_rows.len());
        for (nanoid, name) in tournament_rows {
            let is_participant = is_tournament_participant(&mut conn, user_id, &nanoid)
                .await
                .unwrap_or(false);
            let muted = muted_nanoids.contains(&nanoid);
            out.push(TournamentChannel {
                nanoid,
                name,
                is_participant,
                muted,
            });
        }
        out
    };

    let games = get_game_channels_for_user(&mut conn, user_id)
        .await
        .map_err(ServerFnError::new)?
        .into_iter()
        .map(|(channel_type, channel_id, label, white_id, black_id, finished)| GameChannel {
            channel_type,
            channel_id,
            label,
            white_id,
            black_id,
            finished,
        })
        .collect();

    let has_global =
        global_channel_has_messages(&mut conn).await.unwrap_or(false);

    Ok(MyConversations {
        dms,
        tournaments,
        games,
        has_global,
    })
}
