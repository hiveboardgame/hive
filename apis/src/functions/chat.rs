//! Server functions for chat read receipts and unread counts.

#[cfg(feature = "ssr")]
use crate::functions::auth::identity::uuid;
#[cfg(feature = "ssr")]
use crate::functions::db::pool;
use chrono::{DateTime, Utc};
#[cfg(feature = "ssr")]
use db_lib::get_conn;
#[cfg(feature = "ssr")]
use db_lib::helpers::{
    can_user_access_chat_channel,
    get_blocked_user_ids,
    get_chat_messages_for_channel,
    get_messages_hub_catalog_for_user,
    get_unread_counts_for_messages_hub_catalog,
    get_unread_counts_for_user,
    upsert_chat_read_receipt,
};
use leptos::prelude::*;
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

#[server]
pub async fn mark_chat_read(channel_type: String, channel_id: String) -> Result<(), ServerFnError> {
    // Trim in case client sends trailing/leading spaces; other endpoints use exact constants.
    let channel_type = channel_type.trim();
    if !shared_types::is_valid_chat_channel_type(channel_type) {
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
pub async fn get_chat_unread_counts() -> Result<Vec<(String, String, i64)>, ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(ServerFnError::new)?;
    get_unread_counts_for_user(&mut conn, user_id)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_messages_hub_data() -> Result<MessagesHubData, ServerFnError> {
    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(ServerFnError::new)?;
    load_messages_hub_data_for_user(&mut conn, user_id)
        .await
        .map_err(ServerFnError::new)
}

#[server]
pub async fn get_chat_history(
    channel_type: String,
    channel_id: String,
    limit: Option<i64>,
    before_id: Option<i64>,
) -> Result<Vec<shared_types::ChatMessage>, ServerFnError> {
    let channel_type = channel_type.trim();
    if !shared_types::is_valid_chat_channel_type(channel_type) {
        return Err(ServerFnError::new("Invalid channel_type"));
    }

    let user_id: uuid::Uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await.map_err(ServerFnError::new)?;

    let requested_limit = limit.unwrap_or(DEFAULT_HISTORY_LIMIT);
    let capped_limit = requested_limit.clamp(1, MAX_HISTORY_LIMIT);
    let (effective_limit, effective_before_id) =
        if channel_type == shared_types::CHANNEL_TYPE_GLOBAL {
            (capped_limit.min(GLOBAL_ANNOUNCEMENTS_LIMIT), None)
        } else {
            (capped_limit, before_id)
        };

    let channel_id = if channel_type == shared_types::CHANNEL_TYPE_DIRECT {
        shared_types::canonicalize_dm_channel_id_for_user(&channel_id, user_id)
            .unwrap_or(channel_id)
    } else {
        channel_id
    };

    let allowed = can_user_access_chat_channel(&mut conn, user_id, channel_type, &channel_id)
        .await
        .map_err(ServerFnError::new)?;
    if !allowed {
        return Err(ServerFnError::new("Access denied"));
    }

    let mut messages = get_chat_messages_for_channel(
        &mut conn,
        channel_type,
        &channel_id,
        effective_limit,
        effective_before_id,
    )
    .await
    .map_err(ServerFnError::new)?;

    if channel_type == shared_types::CHANNEL_TYPE_DIRECT {
        let blocked_ids = get_blocked_user_ids(&mut conn, user_id)
            .await
            .map_err(ServerFnError::new)?;
        let blocked_ids: std::collections::HashSet<_> = blocked_ids.into_iter().collect();
        messages.retain(|message| !blocked_ids.contains(&message.sender_id));
    }

    Ok(messages
        .into_iter()
        .map(|message| shared_types::ChatMessage {
            user_id: message.sender_id,
            username: message.username,
            timestamp: Some(message.created_at),
            message: message.body,
            turn: message.turn.map(|turn| turn as usize),
        })
        .collect())
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
    pub white_id: uuid::Uuid,
    pub black_id: uuid::Uuid,
    /// True when the game is finished; client preloads both channels so players can toggle
    /// between Players/Spectators without an extra fetch.
    pub finished: bool,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MessagesHubData {
    pub conversations: MyConversations,
    pub blocked_user_ids: Vec<uuid::Uuid>,
    pub unread_counts: Vec<(String, String, i64)>,
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
fn build_messages_hub_conversations(
    catalog: db_lib::helpers::MessagesHubCatalog,
    unread_counts: &[(String, String, i64)],
) -> MyConversations {
    let db_lib::helpers::MessagesHubCatalog {
        dms,
        tournaments,
        games,
        has_global,
        ..
    } = catalog;
    let recent_cutoff = Utc::now() - Duration::days(MESSAGES_HUB_RECENT_DAYS);
    let unread_counts = unread_count_map(unread_counts);

    let dms: Vec<DmConversation> = prioritize_unread_then_limit(
        dms.into_iter().filter(|row| {
            should_keep_channel(
                shared_types::CHANNEL_TYPE_DIRECT,
                &row.channel_id,
                row.last_message_at,
                &unread_counts,
                recent_cutoff,
            )
        }),
        MESSAGES_HUB_SECTION_LIMIT,
        |row| {
            channel_unread_count(
                &unread_counts,
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
                &unread_counts,
                recent_cutoff,
            )
        }),
        MESSAGES_HUB_SECTION_LIMIT,
        |row| {
            channel_unread_count(
                &unread_counts,
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
                &unread_counts,
                recent_cutoff,
            )
        }),
        MESSAGES_HUB_SECTION_LIMIT,
        |row| channel_unread_count(&unread_counts, &row.channel_type, &row.channel_id) > 0,
    )
    .into_iter()
    .map(|row| GameChannel {
        channel_type: row.channel_type,
        channel_id: row.channel_id,
        label: row.label,
        white_id: row.white_id,
        black_id: row.black_id,
        finished: row.finished,
        last_message_at: row.last_message_at,
    })
    .collect::<Vec<_>>();

    MyConversations {
        dms,
        tournaments,
        games,
        has_global,
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{build_messages_hub_conversations, GameChannel, TournamentChannel};
    use chrono::{Duration, Utc};
    use db_lib::helpers::{
        DmConversationSummary,
        GameChannelSummary,
        MessagesHubCatalog,
        TournamentChannelSummary,
    };
    use uuid::Uuid;

    use super::MESSAGES_HUB_SECTION_LIMIT;

    fn unread_dm_summary(minutes_ago: i64, label: &str) -> DmConversationSummary {
        DmConversationSummary {
            other_user_id: Uuid::new_v4(),
            username: label.to_string(),
            channel_id: format!("dm-{label}"),
            last_message_at: Utc::now() - Duration::minutes(minutes_ago),
        }
    }

    fn tournament_summary(minutes_ago: i64, label: &str) -> TournamentChannelSummary {
        TournamentChannelSummary {
            nanoid: format!("tournament-{label}"),
            name: label.to_string(),
            is_participant: true,
            muted: false,
            last_message_at: Utc::now() - Duration::minutes(minutes_ago),
        }
    }

    fn game_summary(minutes_ago: i64, label: &str) -> GameChannelSummary {
        GameChannelSummary {
            channel_type: shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string(),
            channel_id: format!("game-{label}"),
            label: label.to_string(),
            white_id: Uuid::new_v4(),
            black_id: Uuid::new_v4(),
            finished: false,
            last_message_at: Utc::now() - Duration::minutes(minutes_ago),
        }
    }

    #[test]
    fn messages_hub_keeps_older_unread_dm_ahead_of_section_cap() {
        let unread = unread_dm_summary(120, "unread");
        let filler = (0..MESSAGES_HUB_SECTION_LIMIT)
            .map(|idx| unread_dm_summary(idx as i64, &format!("recent-{idx}")))
            .collect::<Vec<_>>();
        let unread_channel_id = unread.channel_id.clone();

        let conversations = build_messages_hub_conversations(
            MessagesHubCatalog {
                blocked_user_ids: Vec::new(),
                dms: filler.into_iter().chain(std::iter::once(unread)).collect(),
                tournaments: Vec::new(),
                games: Vec::new(),
                has_global: true,
            },
            &[(
                shared_types::CHANNEL_TYPE_DIRECT.to_string(),
                unread_channel_id.clone(),
                3,
            )],
        );

        assert_eq!(conversations.dms.len(), MESSAGES_HUB_SECTION_LIMIT);
        assert_eq!(
            conversations.dms.first().map(|dm| dm.username.clone()),
            Some("unread".to_string())
        );
    }

    #[test]
    fn messages_hub_keeps_older_unread_tournament_and_game_ahead_of_section_cap() {
        let unread_tournament = tournament_summary(120, "unread-tournament");
        let unread_game = game_summary(120, "unread-game");
        let tournament_id = unread_tournament.nanoid.clone();
        let game_id = unread_game.channel_id.clone();

        let conversations = build_messages_hub_conversations(
            MessagesHubCatalog {
                blocked_user_ids: Vec::new(),
                dms: Vec::new(),
                tournaments: (0..MESSAGES_HUB_SECTION_LIMIT)
                    .map(|idx| tournament_summary(idx as i64, &format!("recent-tournament-{idx}")))
                    .chain(std::iter::once(unread_tournament))
                    .collect(),
                games: (0..MESSAGES_HUB_SECTION_LIMIT)
                    .map(|idx| game_summary(idx as i64, &format!("recent-game-{idx}")))
                    .chain(std::iter::once(unread_game))
                    .collect(),
                has_global: true,
            },
            &[
                (
                    shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY.to_string(),
                    tournament_id.clone(),
                    1,
                ),
                (
                    shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string(),
                    game_id.clone(),
                    2,
                ),
            ],
        );

        assert_eq!(
            conversations
                .tournaments
                .first()
                .map(channel_id_for_tournament),
            Some(tournament_id)
        );
        assert_eq!(
            conversations.games.first().map(channel_id_for_game),
            Some(game_id)
        );
    }

    fn channel_id_for_tournament(tournament: &TournamentChannel) -> String {
        tournament.nanoid.clone()
    }

    fn channel_id_for_game(game: &GameChannel) -> String {
        game.channel_id.clone()
    }
}

#[cfg(feature = "ssr")]
async fn load_messages_hub_data_for_user(
    conn: &mut db_lib::DbConn<'_>,
    user_id: uuid::Uuid,
) -> Result<MessagesHubData, db_lib::db_error::DbError> {
    let catalog = get_messages_hub_catalog_for_user(conn, user_id).await?;
    let unread_counts = get_unread_counts_for_messages_hub_catalog(conn, user_id, &catalog).await?;
    let blocked_user_ids = catalog.blocked_user_ids.clone();
    let conversations = build_messages_hub_conversations(catalog, &unread_counts);
    Ok(MessagesHubData {
        conversations,
        blocked_user_ids,
        unread_counts,
    })
}
