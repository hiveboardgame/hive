#[cfg(feature = "ssr")]
use crate::{
    chat::access::{allows_anonymous_chat_read, authorize_chat_read, ChatAccessError},
    functions::{auth::identity::uuid, db::pool},
};
#[cfg(feature = "ssr")]
use db_lib::{
    db_error::DbError,
    get_conn,
    helpers::{
        blocked_user_ids,
        chat_inbox_unread_states,
        get_dm_conversations_for_user,
        get_game_channels_for_user,
        get_tournament_channels_for_user,
        get_tournament_thread_data as db_get_tournament_thread_data,
        is_tournament_chat_muted,
        load_chat_history,
        load_game_chat_capabilities,
        mark_chat_read as db_mark_chat_read,
        muted_tournament_ids_for_user,
        unread_chat_count_for_channel,
        DbChatTarget,
    },
    models::User,
    DbConn,
    DbPool,
};
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use log::error;
use serde::{Deserialize, Serialize};
use server_fn::codec;
use shared_types::{
    ChatHistoryResponse,
    ChatInboxSnapshot,
    ConversationKey,
    GameChatCapabilities,
    GameId,
    MessagesCatalogData,
};
use uuid::Uuid;

#[cfg(any(feature = "ssr", test))]
const HISTORY_PAGE_SIZE: i64 = 50;
#[cfg(any(feature = "ssr", test))]
const MAX_INITIAL_HISTORY_MESSAGES: i64 = 200;
#[cfg(any(feature = "ssr", test))]
const GLOBAL_HISTORY_LIMIT: i64 = 3;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TournamentRouteResponse {
    Ready(String),
    NotFound,
    AccessDenied,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameChatRouteResponse {
    Ready(GameChatCapabilities),
    NotFound,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DmRouteResponse {
    Ready {
        other_user_id: Uuid,
        username: String,
        peer_deleted: bool,
    },
    NotFound,
}

#[cfg(any(feature = "ssr", test))]
fn history_page_limit(
    channel_key: &ConversationKey,
    before_message_id: Option<i64>,
    initial_unread_count: Option<i64>,
) -> i64 {
    if matches!(channel_key, ConversationKey::Global) {
        return GLOBAL_HISTORY_LIMIT;
    }
    if before_message_id.is_some() {
        return HISTORY_PAGE_SIZE;
    }
    initial_unread_count
        .unwrap_or(HISTORY_PAGE_SIZE)
        .clamp(HISTORY_PAGE_SIZE, MAX_INITIAL_HISTORY_MESSAGES)
}

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
async fn chat_connection(pool: &DbPool) -> Result<DbConn<'_>, ServerFnError> {
    get_conn(pool)
        .await
        .map_err(|err| chat_unexpected_error("getting database connection", err))
}

#[cfg(feature = "ssr")]
async fn dispatch_chat_read_update(user_id: Uuid, key: ConversationKey, last_read_message_id: i64) {
    use crate::{
        common::{ServerMessage, ServerResult},
        notifications::AckKey,
        websocket::{MessageDestination, WebsocketData, WsHub},
    };
    use actix_web::web::Data;
    use bytes::Bytes;
    use codee::{binary::MsgpackSerdeCodec, Encoder};

    // A DM read receipt is the same source of truth the header bell uses to
    // clear its unread state, so it also cancels any DM push notification
    // still parked (`Notifier`'s ack/park window, see `ChatHandler::dispatch_notifications`)
    // waiting to fire for this conversation.
    if matches!(key, ConversationKey::Direct(_)) {
        if let Ok(data) = leptos_actix::extract::<Data<WebsocketData>>().await {
            data.pending_notifications
                .mark_seen(user_id, &AckKey::Chat(key.clone()));
        }
    }

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
    hub.dispatch(&MessageDestination::User(user_id), Bytes::from(serialized))
        .await;
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_chat_history(
    channel_key: ConversationKey,
    before_message_id: Option<i64>,
) -> Result<ChatHistoryResponse, ServerFnError> {
    let user_id = match uuid().await {
        Ok(user_id) => Some(user_id),
        Err(_) if allows_anonymous_chat_read(&channel_key) => None,
        Err(error) => return Err(error),
    };
    let pool = pool().await?;
    let mut conn = chat_connection(&pool).await?;
    let target = match authorize_chat_read(&mut conn, user_id, &channel_key).await {
        Ok(target) => target,
        Err(ChatAccessError::Denied) => return Ok(ChatHistoryResponse::AccessDenied),
        Err(error) => return Err(chat_access_check_error("checking chat access", error)),
    };

    let muted = if before_message_id.is_none() {
        match (&target, user_id) {
            (DbChatTarget::Tournament { id, .. }, Some(user_id)) => {
                is_tournament_chat_muted(&mut conn, user_id, *id)
                    .await
                    .map_err(|err| chat_unexpected_error("loading tournament mute state", err))?
            }
            _ => false,
        }
    } else {
        false
    };
    let initial_unread_count =
        if before_message_id.is_none() && channel_key.tracks_read_receipts() && !muted {
            match (user_id, target.channel_id()) {
                (Some(user_id), Some(channel_id)) => Some(
                    unread_chat_count_for_channel(&mut conn, user_id, channel_id)
                        .await
                        .map_err(|err| chat_unexpected_error("loading chat unread count", err))?,
                ),
                (Some(_), None) => Some(0),
                (None, _) => None,
            }
        } else {
            None
        };
    // A persisted send is the sender's read anchor: insertion and receipt advancement are
    // atomic, so own messages cannot remain above the sender's receipt. Initial history is
    // intentionally sized from the unread count up to a 200-message safety cap. Reaching the
    // bottom after a larger backlog marks the omitted messages read. Messages committed between
    // the count and page queries may also shift the boundary. Both are accepted product
    // tradeoffs for current chat traffic.
    let effective_limit = history_page_limit(&channel_key, before_message_id, initial_unread_count);
    let mut page = load_chat_history(&mut conn, &target, before_message_id, effective_limit)
        .await
        .map_err(|err| chat_unexpected_error("loading chat history", err))?;
    page.initial_unread_count = initial_unread_count;
    if matches!(channel_key, ConversationKey::Global) {
        page.next_before_message_id = None;
    }
    Ok(ChatHistoryResponse::Page(page))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn mark_chat_read(
    channel_key: ConversationKey,
    last_read_message_id: i64,
) -> Result<i64, ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = chat_connection(&pool).await?;
    let target = match authorize_chat_read(&mut conn, Some(user_id), &channel_key).await {
        Ok(target) => target,
        Err(ChatAccessError::Denied) => return Err(ServerFnError::new("Access denied")),
        Err(error) => return Err(chat_access_check_error("checking chat access", error)),
    };
    let marked_read_through = db_mark_chat_read(&mut conn, user_id, &target, last_read_message_id)
        .await
        .map_err(|err| chat_unexpected_error("marking chat read", err))?;
    dispatch_chat_read_update(user_id, channel_key, marked_read_through).await;
    Ok(marked_read_through)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_chat_inbox_snapshot() -> Result<ChatInboxSnapshot, ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = chat_connection(&pool).await?;
    let blocked_user_ids = blocked_user_ids(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading blocked users", err))?;
    let muted_tournament_ids = muted_tournament_ids_for_user(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading muted tournament chats", err))?;
    let unread_states = chat_inbox_unread_states(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading chat inbox unread state", err))?;
    Ok(ChatInboxSnapshot {
        blocked_user_ids,
        muted_tournament_ids,
        unread_states,
    })
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_messages_catalog_data() -> Result<MessagesCatalogData, ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = chat_connection(&pool).await?;
    let dms = get_dm_conversations_for_user(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading direct conversations", err))?;
    let tournaments = get_tournament_channels_for_user(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading tournament conversations", err))?;
    let games = get_game_channels_for_user(&mut conn, user_id)
        .await
        .map_err(|err| chat_unexpected_error("loading game conversations", err))?;
    let data = MessagesCatalogData {
        dms,
        tournaments,
        games,
    };
    Ok(data)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn resolve_dm_route_user(username: String) -> Result<DmRouteResponse, ServerFnError> {
    let _user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = chat_connection(&pool).await?;
    match User::find_dm_route_user_by_username(username.trim(), &mut conn)
        .await
        .map_err(|err| chat_unexpected_error("resolving direct-message user", err))?
    {
        Some((other_user_id, username, peer_deleted)) => Ok(DmRouteResponse::Ready {
            other_user_id,
            username,
            peer_deleted,
        }),
        None => Ok(DmRouteResponse::NotFound),
    }
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_tournament_route_data(
    tournament_id: String,
) -> Result<TournamentRouteResponse, ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = chat_connection(&pool).await?;
    match db_get_tournament_thread_data(&mut conn, user_id, tournament_id.trim()).await {
        Ok(name) => Ok(TournamentRouteResponse::Ready(name)),
        Err(DbError::Unauthorized) => Ok(TournamentRouteResponse::AccessDenied),
        Err(DbError::NotFound { .. } | DbError::InvalidInput { .. }) => {
            Ok(TournamentRouteResponse::NotFound)
        }
        Err(error) => Err(chat_user_action_error(
            "loading tournament chat route data",
            error,
        )),
    }
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_game_chat_route_data(
    game_id: GameId,
) -> Result<GameChatRouteResponse, ServerFnError> {
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = chat_connection(&pool).await?;
    match load_game_chat_capabilities(&mut conn, user_id, &game_id)
        .await
        .map_err(|err| chat_unexpected_error("loading game chat capabilities", err))?
    {
        Some(access) => Ok(GameChatRouteResponse::Ready(access)),
        None => Ok(GameChatRouteResponse::NotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::{history_page_limit, HISTORY_PAGE_SIZE};
    use shared_types::ConversationKey;
    use uuid::Uuid;

    #[test]
    fn initial_history_limit_is_unread_aware_with_fifty_to_two_hundred_bounds() {
        let key = ConversationKey::direct(Uuid::new_v4());

        assert_eq!(history_page_limit(&key, None, None), HISTORY_PAGE_SIZE);
        assert_eq!(history_page_limit(&key, None, Some(0)), HISTORY_PAGE_SIZE);
        assert_eq!(history_page_limit(&key, None, Some(49)), HISTORY_PAGE_SIZE);
        assert_eq!(history_page_limit(&key, None, Some(73)), 73);
        assert_eq!(history_page_limit(&key, None, Some(250)), 200);
    }
}
