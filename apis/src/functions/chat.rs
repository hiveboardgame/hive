//! Server functions for chat read receipts and unread counts.

#[cfg(feature = "ssr")]
use crate::functions::auth::identity::uuid;
#[cfg(feature = "ssr")]
use crate::functions::db::pool;
use chrono::{DateTime, Utc};
#[cfg(feature = "ssr")]
use log::error;
#[cfg(feature = "ssr")]
use db_lib::get_conn;
#[cfg(feature = "ssr")]
use db_lib::helpers::{
    can_user_access_chat_channel,
    get_blocked_user_ids,
    get_chat_messages_for_channel,
    get_game_chat_participants_and_finished,
    get_messages_hub_catalog_for_user,
    get_tournament_name_by_nanoid,
    is_tournament_participant,
    is_tournament_chat_muted,
    get_unread_counts_for_messages_hub_catalog,
    get_unread_counts_for_user,
    upsert_chat_read_receipt,
};
use leptos::prelude::*;
use server_fn::codec;
use shared_types::GameId;
#[cfg(feature = "ssr")]
use shared_types::{
    ChannelKey,
    ChannelType,
    ChatMessage,
};
#[cfg(feature = "ssr")]
use std::collections::HashMap;

#[cfg(feature = "ssr")]
use chrono::Duration;
#[cfg(feature = "ssr")]
const DEFAULT_HISTORY_LIMIT: i64 = 50;
#[cfg(feature = "ssr")]
const MAX_HISTORY_LIMIT: i64 = 100;
#[cfg(feature = "ssr")]
const GLOBAL_ANNOUNCEMENTS_LIMIT: i64 = 3;
#[cfg(feature = "ssr")]
const MESSAGES_HUB_RECENT_DAYS: i64 = 30;
#[cfg(feature = "ssr")]
const MESSAGES_HUB_SECTION_LIMIT: usize = 25;

#[cfg(feature = "ssr")]
fn generic_chat_server_error(context: &'static str, err: impl std::fmt::Display) -> ServerFnError {
    error!("chat server fn failed while {context}: {err}");
    ServerFnError::new("Unable to complete chat request")
}

#[cfg(feature = "ssr")]
fn normalize_requested_channel(
    channel_type: &str,
    channel_id: &str,
) -> Result<ChannelKey, ServerFnError> {
    let channel_type = channel_type
        .trim()
        .parse::<ChannelType>()
        .map_err(|_| ServerFnError::new("Invalid channel_type"))?;
    ChannelKey::normalized(channel_type, channel_id)
        .ok_or_else(|| ServerFnError::new("Invalid channel_id"))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn mark_chat_read(channel_type: String, channel_id: String) -> Result<(), ServerFnError> {
    let channel_key = normalize_requested_channel(&channel_type, &channel_id)?;
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;

    // Verify user can access this channel before creating a read receipt
    let allowed = can_user_access_chat_channel(
        &mut conn,
        user_id,
        channel_key.channel_type.as_str(),
        &channel_key.channel_id,
    )
    .await
    .map_err(|err| generic_chat_server_error("checking chat access", err))?;
    if !allowed {
        return Err(ServerFnError::new("Access denied"));
    }

    upsert_chat_read_receipt(
        &mut conn,
        user_id,
        channel_key.channel_type.as_str(),
        &channel_key.channel_id,
        Utc::now(),
    )
    .await
    .map_err(|err| generic_chat_server_error("marking chat as read", err))?;
    Ok(())
}

/// Returns (channel_type, channel_id, unread_count) for channels where the user has a receipt and count > 0.
#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_chat_unread_counts() -> Result<Vec<(String, String, i64)>, ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;
    get_unread_counts_for_user(&mut conn, user_id)
        .await
        .map_err(|err| generic_chat_server_error("loading unread chat counts", err))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_messages_hub_data() -> Result<MessagesHubData, ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;
    load_messages_hub_data_for_user(&mut conn, user_id)
        .await
        .map_err(|err| generic_chat_server_error("loading messages hub data", err))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_chat_history(
    channel_type: String,
    channel_id: String,
    limit: Option<i64>,
) -> Result<Vec<shared_types::ChatMessage>, ServerFnError> {
    let channel_key = normalize_requested_channel(&channel_type, &channel_id)?;
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;

    let requested_limit = limit.unwrap_or(DEFAULT_HISTORY_LIMIT);
    let capped_limit = requested_limit.clamp(1, MAX_HISTORY_LIMIT);
    let effective_limit = if channel_key.channel_type == ChannelType::Global {
        capped_limit.min(GLOBAL_ANNOUNCEMENTS_LIMIT)
    } else {
        capped_limit
    };

    let allowed = can_user_access_chat_channel(
        &mut conn,
        user_id,
        channel_key.channel_type.as_str(),
        &channel_key.channel_id,
    )
    .await
    .map_err(|err| generic_chat_server_error("checking chat access", err))?;
    if !allowed {
        return Err(ServerFnError::new("Access denied"));
    }

    let mut messages = get_chat_messages_for_channel(
        &mut conn,
        channel_key.channel_type.as_str(),
        &channel_key.channel_id,
        effective_limit,
    )
    .await
    .map_err(|err| generic_chat_server_error("loading chat history", err))?;

    if channel_key.channel_type == ChannelType::Direct {
        let blocked_ids = get_blocked_user_ids(&mut conn, user_id)
            .await
            .map_err(|err| generic_chat_server_error("loading blocked users", err))?;
        let blocked_ids: std::collections::HashSet<_> = blocked_ids.into_iter().collect();
        messages.retain(|message| !blocked_ids.contains(&message.sender_id));
    }

    Ok(messages
        .into_iter()
        .map(|message| ChatMessage {
            user_id: message.sender_id,
            username: message.username,
            timestamp: Some(message.created_at),
            message: message.body,
            turn: message.turn.map(|turn| turn as usize),
        })
        .collect())
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_tournament_route_data(
    tournament_id: String,
) -> Result<TournamentRouteData, ServerFnError> {
    let tournament_id = tournament_id.trim().to_string();
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;
    let name = get_tournament_name_by_nanoid(&mut conn, &tournament_id)
        .await
        .map_err(|err| generic_chat_server_error("loading tournament route data", err))?;
    let is_participant = is_tournament_participant(&mut conn, user_id, &tournament_id)
        .await
        .map_err(|err| generic_chat_server_error("checking tournament participation", err))?;
    let muted = is_tournament_chat_muted(&mut conn, user_id, &tournament_id)
        .await
        .map_err(|err| generic_chat_server_error("loading tournament mute state", err))?;

    Ok(TournamentRouteData {
        name,
        is_participant,
        muted,
    })
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_chat_route_data(
    game_id: GameId,
) -> Result<GameChatRouteData, ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;
    let (white_id, black_id, finished) =
        get_game_chat_participants_and_finished(&mut conn, &game_id)
        .await
        .map_err(|err| generic_chat_server_error("loading game chat route data", err))?;

    Ok(GameChatRouteData {
        is_player: user_id == white_id || user_id == black_id,
        finished,
    })
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DmConversation {
    pub other_user_id: uuid::Uuid,
    pub username: String,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TournamentChannel {
    pub nanoid: String,
    pub name: String,
    /// True if the current user is a participant (player or organizer) and can send messages.
    pub is_participant: bool,
    /// True if the user has muted this tournament's lobby chat (no live push, no unread).
    pub muted: bool,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameChannel {
    pub channel_type: String,
    pub channel_id: String,
    pub label: String,
    pub is_player: bool,
    /// True when the game is finished; client preloads both channels so players can toggle
    /// between Players/Spectators without an extra fetch.
    pub finished: bool,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MessagesHubData {
    pub dms: Vec<DmConversation>,
    pub tournaments: Vec<TournamentChannel>,
    pub games: Vec<GameChannel>,
    pub unread_counts: Vec<(String, String, i64)>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TournamentRouteData {
    pub name: String,
    pub is_participant: bool,
    pub muted: bool,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameChatRouteData {
    pub is_player: bool,
    pub finished: bool,
}

#[cfg(feature = "ssr")]
fn unread_count_map(unread_counts: &[(String, String, i64)]) -> HashMap<(String, String), i64> {
    unread_counts
        .iter()
        .map(|(channel_type, channel_id, count)| {
            ((channel_type.clone(), channel_id.clone()), *count)
        })
        .collect()
}

#[cfg(feature = "ssr")]
fn should_keep_channel(
    channel_type: &str,
    channel_id: &str,
    last_message_at: DateTime<Utc>,
    unread_counts: &HashMap<(String, String), i64>,
    recent_cutoff: DateTime<Utc>,
) -> bool {
    last_message_at >= recent_cutoff
        || unread_counts
            .get(&(channel_type.to_string(), channel_id.to_string()))
            .copied()
            .unwrap_or(0)
            > 0
}

#[cfg(feature = "ssr")]
fn channel_unread_count(
    unread_counts: &HashMap<(String, String), i64>,
    channel_type: &str,
    channel_id: &str,
) -> i64 {
    unread_counts
        .get(&(channel_type.to_string(), channel_id.to_string()))
        .copied()
        .unwrap_or(0)
}

#[cfg(feature = "ssr")]
fn prioritize_unread_then_limit<T>(
    rows: impl IntoIterator<Item = T>,
    limit: usize,
    has_unread: impl Fn(&T) -> bool,
) -> Vec<T> {
    let (unread, read): (Vec<_>, Vec<_>) = rows.into_iter().partition(|row| has_unread(row));
    let read_limit = limit.saturating_sub(unread.len());
    unread
        .into_iter()
        .chain(read.into_iter().take(read_limit))
        .collect()
}

#[cfg(feature = "ssr")]
fn build_messages_hub_data(
    catalog: db_lib::helpers::MessagesHubCatalog,
    unread_counts: Vec<(String, String, i64)>,
    user_id: uuid::Uuid,
) -> MessagesHubData {
    let db_lib::helpers::MessagesHubCatalog {
        dms,
        tournaments,
        games,
        ..
    } = catalog;
    let recent_cutoff = Utc::now() - Duration::days(MESSAGES_HUB_RECENT_DAYS);
    let unread_count_map = unread_count_map(&unread_counts);

    let dms: Vec<DmConversation> = prioritize_unread_then_limit(
        dms.into_iter().filter(|row| {
            should_keep_channel(
                shared_types::CHANNEL_TYPE_DIRECT,
                &row.channel_id,
                row.last_message_at,
                &unread_count_map,
                recent_cutoff,
            )
        }),
        MESSAGES_HUB_SECTION_LIMIT,
        |row| {
            channel_unread_count(
                &unread_count_map,
                shared_types::CHANNEL_TYPE_DIRECT,
                &row.channel_id,
            ) > 0
        },
    )
    .into_iter()
    .map(|row| DmConversation {
        other_user_id: row.other_user_id,
        username: row.username,
        last_message_at: row.last_message_at,
    })
    .collect();

    let tournaments: Vec<TournamentChannel> = prioritize_unread_then_limit(
        tournaments.into_iter().filter(|row| {
            should_keep_channel(
                shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY,
                &row.nanoid,
                row.last_message_at,
                &unread_count_map,
                recent_cutoff,
            )
        }),
        MESSAGES_HUB_SECTION_LIMIT,
        |row| {
            channel_unread_count(
                &unread_count_map,
                shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY,
                &row.nanoid,
            ) > 0
        },
    )
    .into_iter()
    .map(|row| TournamentChannel {
        nanoid: row.nanoid,
        name: row.name,
        is_participant: row.is_participant,
        muted: row.muted,
        last_message_at: row.last_message_at,
    })
    .collect::<Vec<_>>();

    let games: Vec<GameChannel> = prioritize_unread_then_limit(
        games.into_iter().filter(|row| {
            should_keep_channel(
                &row.channel_type,
                &row.channel_id,
                row.last_message_at,
                &unread_count_map,
                recent_cutoff,
            )
        }),
        MESSAGES_HUB_SECTION_LIMIT,
        |row| channel_unread_count(&unread_count_map, &row.channel_type, &row.channel_id) > 0,
    )
    .into_iter()
    .map(|row| GameChannel {
        channel_type: row.channel_type,
        channel_id: row.channel_id,
        label: row.label,
        is_player: row.white_id == user_id || row.black_id == user_id,
        finished: row.finished,
        last_message_at: row.last_message_at,
    })
    .collect::<Vec<_>>();

    MessagesHubData {
        dms,
        tournaments,
        games,
        unread_counts,
    }
}

#[cfg(feature = "ssr")]
async fn load_messages_hub_data_for_user(
    conn: &mut db_lib::DbConn<'_>,
    user_id: uuid::Uuid,
) -> Result<MessagesHubData, db_lib::db_error::DbError> {
    let catalog = get_messages_hub_catalog_for_user(conn, user_id).await?;
    let unread_counts = get_unread_counts_for_messages_hub_catalog(conn, user_id, &catalog).await?;
    Ok(build_messages_hub_data(catalog, unread_counts, user_id))
}
