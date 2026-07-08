#[cfg(feature = "ssr")]
use crate::{
    chat::access::{
        allows_anonymous_chat_read,
        can_anonymous_read_chat,
        can_user_read_chat,
        ChatAccessError,
    },
    functions::{auth::identity::uuid, db::pool},
};
#[cfg(feature = "ssr")]
use db_lib::{
    db_error::DbError,
    get_conn,
    helpers::{
        get_dm_conversations_for_user,
        get_game_channels_for_user,
        get_tournament_channels_for_user,
        get_tournament_thread_data as db_get_tournament_thread_data,
        load_chat_history,
        load_game_chat_capabilities,
        mark_chat_read as db_mark_chat_read,
        unread_states_for_messages_hub_channels,
    },
    models::UserTournamentChatMute,
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
};
#[cfg(feature = "ssr")]
use uuid::Uuid;

#[cfg(feature = "ssr")]
const DEFAULT_HISTORY_LIMIT: i64 = 50;
#[cfg(feature = "ssr")]
const MAX_HISTORY_LIMIT: i64 = 100;
#[cfg(feature = "ssr")]
const GLOBAL_HISTORY_LIMIT: i64 = 3;

#[cfg(feature = "ssr")]
fn chat_unexpected_error(context: &'static str, err: impl std::fmt::Display) -> ServerFnError {
    error!("chat server function failed while {context}: {err}");
    ServerFnError::new("Unable to complete chat request")
}

#[cfg(feature = "ssr")]
fn chat_access_check_error(context: &'static str, err: ChatAccessError) -> ServerFnError {
    if let ChatAccessError::Internal {
        context: inner_context,
        error,
    } = err
    {
        error!("chat server function failed while {context}: {inner_context}: {error}");
    }
    ServerFnError::new("Unable to complete chat request")
}

#[cfg(feature = "ssr")]
fn chat_user_action_error(context: &'static str, err: DbError) -> ServerFnError {
    if !matches!(
        err,
        DbError::InvalidInput { .. } | DbError::NotFound { .. } | DbError::Unauthorized
    ) {
        error!("chat server function failed while {context}: {err}");
    }
    ServerFnError::new("Unable to complete chat request")
}

#[cfg(feature = "ssr")]
async fn dispatch_chat_read_update(user_id: Uuid, key: ConversationKey, last_read_message_id: i64) {
    use crate::{
        common::{ServerMessage, ServerResult},
        websocket::{MessageDestination, WsHub},
    };
    use actix_web::web::Data;
    use bytes::Bytes;
    use codee::{binary::MsgpackSerdeCodec, Encoder};

    let Ok(hub) = leptos_actix::extract::<Data<std::sync::Arc<WsHub>>>().await else {
        return;
    };
    let message = ServerResult::Ok(Box::new(ServerMessage::ChatRead {
        key,
        last_read_message_id,
    }));
    let Ok(serialized) = MsgpackSerdeCodec::encode(&message) else {
        return;
    };
    hub.dispatch(
        &MessageDestination::User(user_id),
        Bytes::from(serialized),
        None,
    )
    .await;
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_chat_history(
    channel_key: ConversationKey,
    limit: Option<i64>,
) -> Result<ChatHistoryResponse, ServerFnError> {
    let user_id = match uuid().await {
        Ok(user_id) => Some(user_id),
        Err(_) if allows_anonymous_chat_read(&channel_key) => None,
        Err(error) => return Err(error),
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| chat_unexpected_error("getting database connection", err))?;
    let (allowed, target) = if let Some(user_id) = user_id {
        can_user_read_chat(&mut conn, user_id, &channel_key)
            .await
            .map_err(|err| chat_access_check_error("checking chat access", err))?
    } else {
        can_anonymous_read_chat(&mut conn, &channel_key)
            .await
            .map_err(|err| chat_access_check_error("checking anonymous chat access", err))?
    };
    if !allowed {
        return Ok(ChatHistoryResponse::AccessDenied);
    }

    let requested_limit = limit.unwrap_or(DEFAULT_HISTORY_LIMIT);
    let mut effective_limit = requested_limit.clamp(1, MAX_HISTORY_LIMIT);
    if matches!(channel_key, ConversationKey::Global) {
        effective_limit = effective_limit.min(GLOBAL_HISTORY_LIMIT);
    }
    load_chat_history(&mut conn, &target, effective_limit)
        .await
        .map(ChatHistoryResponse::Messages)
        .map_err(|err| chat_unexpected_error("loading chat history", err))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn mark_chat_read(
    channel_key: ConversationKey,
    last_read_message_id: i64,
) -> Result<(), ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| chat_unexpected_error("getting database connection", err))?;
    let (allowed, target) = can_user_read_chat(&mut conn, user_id, &channel_key)
        .await
        .map_err(|err| chat_access_check_error("checking chat access", err))?;
    if !allowed {
        return Err(ServerFnError::new("Access denied"));
    }
    let marked_read_through = db_mark_chat_read(&mut conn, user_id, &target, last_read_message_id)
        .await
        .map_err(|err| chat_unexpected_error("marking chat read", err))?;
    dispatch_chat_read_update(user_id, channel_key, marked_read_through).await;
    Ok(())
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_messages_hub_data() -> Result<MessagesHubData, ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| chat_unexpected_error("getting database connection", err))?;
    let dms = get_dm_conversations_for_user(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading direct conversations", err))?;
    let tournaments = get_tournament_channels_for_user(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading tournament conversations", err))?;
    let games = get_game_channels_for_user(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading game conversations", err))?;
    let muted_tournament_ids =
        UserTournamentChatMute::muted_tournament_ids_for_user(&mut conn, user_id)
            .await
            .map_err(|err| chat_unexpected_error("loading muted tournament chats", err))?;
    let unread_states = unread_states_for_messages_hub_channels(
        &mut conn,
        user_id,
        &dms,
        &tournaments,
        &games,
        &muted_tournament_ids,
    )
    .await
    .map_err(|err| chat_unexpected_error("loading unread state", err))?;

    Ok(MessagesHubData {
        dms,
        tournaments,
        games,
        muted_tournament_ids,
        unread_states,
    })
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_tournament_route_data(
    tournament_id: String,
) -> Result<(String, bool, TournamentChatCapabilities), ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| chat_unexpected_error("getting database connection", err))?;
    db_get_tournament_thread_data(&mut conn, user_id, tournament_id.trim())
        .await
        .map_err(|err| chat_user_action_error("loading tournament chat route data", err))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_chat_route_data(
    game_id: GameId,
) -> Result<GameChatCapabilities, ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| chat_unexpected_error("getting database connection", err))?;
    load_game_chat_capabilities(&mut conn, user_id, &game_id)
        .await
        .map_err(|err| chat_unexpected_error("loading game chat capabilities", err))?
        .ok_or_else(|| ServerFnError::new("Game not found"))
}
