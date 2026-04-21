//! Server functions for chat read receipts and unread counts.

#[cfg(feature = "ssr")]
use crate::chat::access::{can_user_read_chat, load_game_chat_capabilities};
#[cfg(feature = "ssr")]
use crate::functions::auth::identity::uuid;
#[cfg(feature = "ssr")]
use crate::functions::db::pool;
#[cfg(feature = "ssr")]
use chrono::{DateTime, Utc};
#[cfg(feature = "ssr")]
use db_lib::get_conn;
#[cfg(feature = "ssr")]
use db_lib::helpers::{
    get_chat_messages_for_channel,
    get_dm_conversations_for_user,
    get_game_channels_for_user,
    get_tournament_channels_for_user,
    get_tournament_thread_data,
    get_unread_counts_for_messages_hub_channels,
    upsert_chat_read_receipt,
};
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use log::error;
use server_fn::codec;
use shared_types::{
    ChatHistoryResponse,
    ConversationKey,
    GameChatCapabilities,
    GameId,
    MessagesHubData,
    TournamentChatCapabilities,
    UnreadCount,
};
#[cfg(feature = "ssr")]
use shared_types::{
    ChatMessage,
    DmConversation,
    GameChannel,
    PersistentChannelKey,
    TournamentChannel,
    TournamentId,
};
#[cfg(feature = "ssr")]
use std::collections::HashMap;

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
async fn load_messages_hub_channels(
    conn: &mut db_lib::DbConn<'_>,
    user_id: uuid::Uuid,
) -> Result<
    (
        Vec<DmConversation>,
        Vec<TournamentChannel>,
        Vec<GameChannel>,
    ),
    ServerFnError,
> {
    let dms = get_dm_conversations_for_user(conn, user_id)
        .await
        .map_err(|err| generic_chat_server_error("loading direct message conversations", err))?;
    let tournaments = get_tournament_channels_for_user(conn, user_id)
        .await
        .map_err(|err| generic_chat_server_error("loading tournament channels", err))?;
    let games = get_game_channels_for_user(conn, user_id)
        .await
        .map_err(|err| generic_chat_server_error("loading game channels", err))?;
    Ok((dms, tournaments, games))
}

#[cfg(feature = "ssr")]
fn build_messages_hub_data(
    dms: Vec<DmConversation>,
    tournaments: Vec<TournamentChannel>,
    games: Vec<GameChannel>,
    unread_counts: Vec<UnreadCount>,
    recent_cutoff: DateTime<Utc>,
    section_limit: usize,
) -> MessagesHubData {
    let unread_count_map = unread_count_map(&unread_counts);
    let dms = prioritize_unread_then_limit(
        dms.into_iter().filter(|row| {
            should_keep_channel(
                &ConversationKey::direct(row.other_user_id),
                row.last_message_at,
                &unread_count_map,
                recent_cutoff,
            )
        }),
        section_limit,
        |row| {
            channel_unread_count(
                &unread_count_map,
                &ConversationKey::direct(row.other_user_id),
            ) > 0
        },
    );
    let tournaments = prioritize_unread_then_limit(
        tournaments.into_iter().filter(|row| {
            should_keep_channel(
                &ConversationKey::tournament(&TournamentId(row.nanoid.clone())),
                row.last_message_at,
                &unread_count_map,
                recent_cutoff,
            )
        }),
        section_limit,
        |row| {
            channel_unread_count(
                &unread_count_map,
                &ConversationKey::tournament(&TournamentId(row.nanoid.clone())),
            ) > 0
        },
    );
    let games = prioritize_unread_then_limit(
        games.into_iter().filter(|row| {
            should_keep_channel(
                &ConversationKey::game(&row.game_id, row.thread),
                row.last_message_at,
                &unread_count_map,
                recent_cutoff,
            )
        }),
        section_limit,
        |row| {
            channel_unread_count(
                &unread_count_map,
                &ConversationKey::game(&row.game_id, row.thread),
            ) > 0
        },
    );

    MessagesHubData {
        dms,
        tournaments,
        games,
    }
}

#[cfg(feature = "ssr")]
fn unread_count_map(unread_counts: &[UnreadCount]) -> HashMap<ConversationKey, i64> {
    unread_counts
        .iter()
        .map(|unread| (unread.key.clone(), unread.count))
        .collect()
}

#[cfg(feature = "ssr")]
fn should_keep_channel(
    channel_key: &ConversationKey,
    last_message_at: DateTime<Utc>,
    unread_counts: &HashMap<ConversationKey, i64>,
    recent_cutoff: DateTime<Utc>,
) -> bool {
    last_message_at >= recent_cutoff || channel_unread_count(unread_counts, channel_key) > 0
}

#[cfg(feature = "ssr")]
fn channel_unread_count(
    unread_counts: &HashMap<ConversationKey, i64>,
    channel_key: &ConversationKey,
) -> i64 {
    unread_counts.get(channel_key).copied().unwrap_or(0)
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
fn persistent_channel_key(
    key: &ConversationKey,
    user_id: uuid::Uuid,
) -> Result<PersistentChannelKey, ServerFnError> {
    key.persistent_key(Some(user_id))
        .ok_or_else(|| ServerFnError::new("Invalid channel"))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn mark_chat_read(channel_key: ConversationKey) -> Result<(), ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let persistent_key = persistent_channel_key(&channel_key, user_id)?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;

    // Verify user can access this channel before creating a read receipt
    let allowed = can_user_read_chat(&mut conn, user_id, &channel_key)
        .await
        .map_err(|err| generic_chat_server_error("checking chat access", err))?;
    if !allowed {
        return Err(ServerFnError::new("Access denied"));
    }

    upsert_chat_read_receipt(
        &mut conn,
        user_id,
        persistent_key.channel_type.as_str(),
        &persistent_key.channel_id,
        chrono::Utc::now(),
    )
    .await
    .map_err(|err| generic_chat_server_error("marking chat as read", err))?;
    Ok(())
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_chat_unread_counts() -> Result<Vec<UnreadCount>, ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;
    let (dms, tournaments, games) = load_messages_hub_channels(&mut conn, user_id).await?;
    get_unread_counts_for_messages_hub_channels(&mut conn, user_id, &dms, &tournaments, &games)
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
    let (dms, tournaments, games) = load_messages_hub_channels(&mut conn, user_id).await?;
    let unread_counts =
        get_unread_counts_for_messages_hub_channels(&mut conn, user_id, &dms, &tournaments, &games)
            .await
            .map_err(|err| generic_chat_server_error("loading messages hub unread counts", err))?;
    Ok(build_messages_hub_data(
        dms,
        tournaments,
        games,
        unread_counts,
        chrono::Utc::now() - chrono::Duration::days(MESSAGES_HUB_RECENT_DAYS),
        MESSAGES_HUB_SECTION_LIMIT,
    ))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_chat_history(
    channel_key: ConversationKey,
    limit: Option<i64>,
) -> Result<ChatHistoryResponse, ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let persistent_key = persistent_channel_key(&channel_key, user_id)?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;

    let requested_limit = limit.unwrap_or(DEFAULT_HISTORY_LIMIT);
    let capped_limit = requested_limit.clamp(1, MAX_HISTORY_LIMIT);
    let effective_limit = if matches!(channel_key, ConversationKey::Global) {
        capped_limit.min(GLOBAL_ANNOUNCEMENTS_LIMIT)
    } else {
        capped_limit
    };

    let allowed = can_user_read_chat(&mut conn, user_id, &channel_key)
        .await
        .map_err(|err| generic_chat_server_error("checking chat access", err))?;
    if !allowed {
        return Ok(ChatHistoryResponse::AccessDenied);
    }

    let messages = get_chat_messages_for_channel(
        &mut conn,
        persistent_key.channel_type.as_str(),
        &persistent_key.channel_id,
        effective_limit,
    )
    .await
    .map_err(|err| generic_chat_server_error("loading chat history", err))?;

    Ok(ChatHistoryResponse::Messages(
        messages
            .into_iter()
            .map(|message| ChatMessage {
                user_id: message.sender_id,
                username: message.username,
                timestamp: Some(message.created_at),
                message: message.body,
                turn: message.turn.map(|turn| turn as usize),
            })
            .collect(),
    ))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_tournament_route_data(
    tournament_id: String,
) -> Result<(String, bool, TournamentChatCapabilities), ServerFnError> {
    let tournament_id = tournament_id.trim().to_string();
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;
    get_tournament_thread_data(&mut conn, user_id, &tournament_id)
        .await
        .map_err(|err| generic_chat_server_error("loading tournament route data", err))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_chat_route_data(
    game_id: GameId,
) -> Result<GameChatCapabilities, ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_chat_server_error("getting a database connection", err))?;
    load_game_chat_capabilities(&mut conn, user_id, &game_id)
        .await
        .map_err(|err| generic_chat_server_error("loading game chat route data", err))?
        .ok_or_else(|| ServerFnError::new("Game not found"))
}
