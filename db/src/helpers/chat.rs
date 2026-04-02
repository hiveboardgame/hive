use crate::{
    db_error::DbError,
    models::{ChatMessage, Game, NewChatMessage},
    schema::{
        chat_messages,
        chat_read_receipts,
        games,
        tournaments,
        tournaments_organizers,
        tournaments_users,
        users,
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{
    dsl::{exists, max, sql},
    prelude::*,
    select,
    sql_types::Timestamptz,
};
use diesel_async::RunQueryDsl;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

diesel::allow_columns_to_appear_in_same_group_by_clause!(
    chat_messages::channel_type,
    chat_messages::channel_id,
    games::white_id,
    games::black_id,
    games::finished,
);

diesel::allow_columns_to_appear_in_same_group_by_clause!(tournaments::nanoid, tournaments::name,);

#[derive(Clone, Debug)]
pub struct DmConversationSummary {
    pub other_user_id: Uuid,
    pub username: String,
    pub channel_id: String,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct TournamentChannelSummary {
    pub nanoid: String,
    pub name: String,
    pub is_participant: bool,
    pub muted: bool,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct GameChannelSummary {
    pub channel_type: String,
    pub channel_id: String,
    pub label: String,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub finished: bool,
    pub last_message_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct MessagesHubCatalog {
    pub blocked_user_ids: Vec<Uuid>,
    pub dms: Vec<DmConversationSummary>,
    pub tournaments: Vec<TournamentChannelSummary>,
    pub games: Vec<GameChannelSummary>,
    pub has_global: bool,
}

/// Insert a chat message and return the inserted row.
pub async fn insert_chat_message(
    conn: &mut DbConn<'_>,
    new: NewChatMessage<'_>,
) -> Result<ChatMessage, DbError> {
    let game_id = resolve_game_id_for_channel(conn, new.channel_type, new.channel_id).await?;
    ensure_game_channel_has_game_id(new.channel_type, new.channel_id, &game_id)?;
    NewChatMessage { game_id, ..new }.insert(conn).await
}

/// Load messages for a channel, newest first. `before_id` is for pagination (exclusive).
pub async fn get_chat_messages_for_channel(
    conn: &mut DbConn<'_>,
    channel_type: &str,
    channel_id: &str,
    limit: i64,
    before_id: Option<i64>,
) -> Result<Vec<ChatMessage>, DbError> {
    let mut query = chat_messages::table
        .filter(chat_messages::channel_type.eq(channel_type))
        .filter(chat_messages::channel_id.eq(channel_id))
        .order(chat_messages::created_at.desc())
        .limit(limit)
        .into_boxed();

    if let Some(bid) = before_id {
        query = query.filter(chat_messages::id.lt(bid));
    }

    query.get_results(conn).await.map_err(DbError::from)
}

fn is_game_chat_channel_type(channel_type: &str) -> bool {
    matches!(
        channel_type,
        shared_types::CHANNEL_TYPE_GAME_PLAYERS | shared_types::CHANNEL_TYPE_GAME_SPECTATORS
    )
}

fn ensure_game_channel_has_game_id(
    channel_type: &str,
    channel_id: &str,
    game_id: &Option<Uuid>,
) -> Result<(), DbError> {
    if is_game_chat_channel_type(channel_type) && game_id.is_none() {
        return Err(DbError::NotFound {
            reason: format!("Game chat channel not found: {channel_id}"),
        });
    }
    Ok(())
}

async fn resolve_game_id_for_channel(
    conn: &mut DbConn<'_>,
    channel_type: &str,
    channel_id: &str,
) -> Result<Option<Uuid>, DbError> {
    if !is_game_chat_channel_type(channel_type) {
        return Ok(None);
    }

    match games::table
        .filter(games::nanoid.eq(channel_id))
        .select(games::id)
        .first(conn)
        .await
    {
        Ok(game_id) => Ok(Some(game_id)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(err) => Err(DbError::from(err)),
    }
}

/// Returns true if the global channel has at least one message.
pub async fn global_channel_has_messages(conn: &mut DbConn<'_>) -> Result<bool, DbError> {
    select(exists(
        chat_messages::table
            .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GLOBAL))
            .filter(chat_messages::channel_id.eq(shared_types::CHANNEL_TYPE_GLOBAL)),
    ))
    .get_result(conn)
    .await
    .map_err(DbError::from)
}

/// Returns true if the user is allowed to read from this chat channel.
/// Game chat: only players may ever read game_players.
/// game_spectators is non-player-only while a game is ongoing and globally readable once finished.
pub async fn can_user_access_chat_channel(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    channel_type: &str,
    channel_id: &str,
) -> Result<bool, DbError> {
    match channel_type {
        shared_types::CHANNEL_TYPE_DIRECT => {
            Ok(other_user_from_dm_channel(channel_id, user_id).is_some())
        }
        shared_types::CHANNEL_TYPE_GLOBAL => Ok(true),
        shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY => {
            is_tournament_participant(conn, user_id, channel_id).await
        }
        shared_types::CHANNEL_TYPE_GAME_PLAYERS | shared_types::CHANNEL_TYPE_GAME_SPECTATORS => {
            let game =
                match Game::find_by_game_id(&shared_types::GameId(channel_id.to_string()), conn)
                    .await
                {
                    Ok(g) => g,
                    Err(_) => return Ok(false),
                };
            let is_player = user_id == game.white_id || user_id == game.black_id;

            if channel_type == shared_types::CHANNEL_TYPE_GAME_PLAYERS {
                // Spectators must never read player messages, even after the game is over.
                Ok(is_player)
            } else if game.finished {
                // game_spectators: players may not read while game is ongoing; when finished, anyone may read.
                Ok(true)
            } else {
                Ok(!is_player)
            }
        }
        _ => Ok(false),
    }
}

/// Upsert a read receipt: set last_read_at for (user_id, channel_type, channel_id).
pub async fn upsert_chat_read_receipt(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    channel_type: &str,
    channel_id: &str,
    last_read_at: chrono::DateTime<Utc>,
) -> Result<(), DbError> {
    let game_id = resolve_game_id_for_channel(conn, channel_type, channel_id).await?;
    ensure_game_channel_has_game_id(channel_type, channel_id, &game_id)?;

    diesel::insert_into(chat_read_receipts::table)
        .values((
            chat_read_receipts::user_id.eq(user_id),
            chat_read_receipts::channel_type.eq(channel_type),
            chat_read_receipts::channel_id.eq(channel_id),
            chat_read_receipts::last_read_at.eq(last_read_at),
            chat_read_receipts::game_id.eq(game_id),
        ))
        .on_conflict((
            chat_read_receipts::user_id,
            chat_read_receipts::channel_type,
            chat_read_receipts::channel_id,
        ))
        .do_update()
        .set((
            chat_read_receipts::last_read_at.eq(sql::<Timestamptz>(
                "GREATEST(chat_read_receipts.last_read_at, EXCLUDED.last_read_at)",
            )),
            chat_read_receipts::game_id.eq(game_id),
        ))
        .execute(conn)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

/// Other user id and username for each DM conversation the user has (from chat_messages).
/// Excludes users the current user has blocked and includes the latest persisted activity timestamp.
pub async fn get_dm_conversation_summaries_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    blocked_ids: &HashSet<Uuid>,
) -> Result<Vec<DmConversationSummary>, DbError> {
    use crate::schema::users;

    let channel_activity = get_dm_channel_activity_for_user(conn, user_id).await?;

    let mut other_id_to_activity = HashMap::<Uuid, (String, DateTime<Utc>)>::new();
    for (channel_id, last_message_at) in channel_activity {
        let Some(last_message_at) = last_message_at else {
            continue;
        };
        let Some(other_id) = other_user_from_dm_channel(&channel_id, user_id) else {
            continue;
        };
        if blocked_ids.contains(&other_id) {
            continue;
        }
        match other_id_to_activity.get_mut(&other_id) {
            Some((existing_channel_id, existing_last_message_at)) => {
                if last_message_at > *existing_last_message_at {
                    *existing_channel_id = channel_id;
                    *existing_last_message_at = last_message_at;
                }
            }
            None => {
                other_id_to_activity.insert(other_id, (channel_id, last_message_at));
            }
        }
    }

    if other_id_to_activity.is_empty() {
        return Ok(Vec::new());
    }

    let other_ids: Vec<Uuid> = other_id_to_activity.keys().copied().collect();
    let usernames: Vec<(Uuid, String)> = users::table
        .filter(users::id.eq_any(&other_ids))
        .select((users::id, users::username))
        .load(conn)
        .await
        .map_err(DbError::from)?;
    let username_map: HashMap<Uuid, String> = usernames.into_iter().collect();

    let mut result = other_id_to_activity
        .into_iter()
        .filter_map(|(other_user_id, (channel_id, last_message_at))| {
            username_map
                .get(&other_user_id)
                .cloned()
                .map(|username| DmConversationSummary {
                    other_user_id,
                    username,
                    channel_id,
                    last_message_at,
                })
        })
        .collect::<Vec<_>>();
    result.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));

    Ok(result)
}

fn other_user_from_dm_channel(channel_id: &str, me: Uuid) -> Option<Uuid> {
    shared_types::other_user_from_dm_channel(channel_id, me)
}

fn dm_channel_like_patterns(user_id: Uuid) -> (String, String) {
    shared_types::dm_channel_like_patterns(user_id)
}

async fn get_dm_channel_activity_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(String, Option<DateTime<Utc>>)>, DbError> {
    let (prefix_pattern, suffix_pattern) = dm_channel_like_patterns(user_id);
    chat_messages::table
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_DIRECT))
        .filter(
            chat_messages::channel_id
                .like(prefix_pattern)
                .or(chat_messages::channel_id.like(suffix_pattern)),
        )
        .group_by(chat_messages::channel_id)
        .select((chat_messages::channel_id, max(chat_messages::created_at)))
        .load(conn)
        .await
        .map_err(DbError::from)
}

async fn get_tournament_lobby_activity_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(String, String, Option<DateTime<Utc>>)>, DbError> {
    chat_messages::table
        .inner_join(tournaments::table.on(chat_messages::channel_id.eq(tournaments::nanoid)))
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY))
        .filter(
            exists(
                tournaments_users::table
                    .filter(tournaments_users::user_id.eq(user_id))
                    .filter(tournaments_users::tournament_id.eq(tournaments::id)),
            )
            .or(exists(
                tournaments_organizers::table
                    .filter(tournaments_organizers::organizer_id.eq(user_id))
                    .filter(tournaments_organizers::tournament_id.eq(tournaments::id)),
            )),
        )
        .group_by((tournaments::nanoid, tournaments::name))
        .select((
            tournaments::nanoid,
            tournaments::name,
            max(chat_messages::created_at),
        ))
        .load(conn)
        .await
        .map_err(DbError::from)
}

/// Tournament lobby channels the user has.
/// Includes only participant tournaments with persisted chat activity, plus muted state
/// and latest activity timestamp.
pub async fn get_tournament_lobby_channel_summaries_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    muted_tournament_nanoids: &HashSet<String>,
) -> Result<Vec<TournamentChannelSummary>, DbError> {
    let mut result = get_tournament_lobby_activity_for_user(conn, user_id)
        .await?
        .into_iter()
        .filter_map(|(nanoid, name, last_message_at)| {
            last_message_at.map(|last_message_at| TournamentChannelSummary {
                muted: muted_tournament_nanoids.contains(&nanoid),
                nanoid,
                name,
                is_participant: true,
                last_message_at,
            })
        })
        .collect::<Vec<_>>();
    result.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));
    Ok(result)
}

/// True if the user is a participant in the tournament (player or organizer). Used to allow send in tournament chat.
pub async fn is_tournament_participant(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<bool, DbError> {
    select(exists(
        tournaments::table
            .filter(tournaments::nanoid.eq(tournament_nanoid))
            .filter(
                exists(
                    tournaments_users::table
                        .filter(tournaments_users::user_id.eq(user_id))
                        .filter(tournaments_users::tournament_id.eq(tournaments::id)),
                )
                .or(exists(
                    tournaments_organizers::table
                        .filter(tournaments_organizers::organizer_id.eq(user_id))
                        .filter(tournaments_organizers::tournament_id.eq(tournaments::id)),
                )),
            ),
    ))
    .get_result(conn)
    .await
    .map_err(DbError::from)
}

async fn unread_counts_for_channel_ids(
    conn: &mut DbConn<'_>,
    channel_type: &str,
    channel_ids: &[String],
    user_id: Uuid,
) -> Result<Vec<(String, i64)>, DbError> {
    use diesel::dsl::count_star;

    if channel_ids.is_empty() {
        return Ok(Vec::new());
    }

    chat_messages::table
        .filter(chat_messages::channel_type.eq(channel_type))
        .filter(chat_messages::channel_id.eq_any(channel_ids))
        .filter(chat_messages::sender_id.ne(user_id))
        .group_by(chat_messages::channel_id)
        .select((chat_messages::channel_id, count_star()))
        .load(conn)
        .await
        .map_err(DbError::from)
}

async fn unread_counts_for_receipt_channel_ids(
    conn: &mut DbConn<'_>,
    channel_type: &str,
    channel_ids: &[String],
    user_id: Uuid,
) -> Result<Vec<(String, i64)>, DbError> {
    use diesel::dsl::count_star;

    if channel_ids.is_empty() {
        return Ok(Vec::new());
    }

    chat_messages::table
        .inner_join(
            chat_read_receipts::table.on(chat_messages::channel_type
                .eq(chat_read_receipts::channel_type)
                .and(chat_messages::channel_id.eq(chat_read_receipts::channel_id))),
        )
        .filter(chat_read_receipts::user_id.eq(user_id))
        .filter(chat_messages::channel_type.eq(channel_type))
        .filter(chat_messages::channel_id.eq_any(channel_ids))
        .filter(chat_messages::created_at.gt(chat_read_receipts::last_read_at))
        .filter(chat_messages::sender_id.ne(user_id))
        .group_by(chat_messages::channel_id)
        .select((chat_messages::channel_id, count_star()))
        .load(conn)
        .await
        .map_err(DbError::from)
}

fn extend_unread_counts(
    result: &mut Vec<(String, String, i64)>,
    channel_type: &'static str,
    unread_counts: Vec<(String, i64)>,
) {
    result.extend(
        unread_counts
            .into_iter()
            .map(|(channel_id, count)| (channel_type.to_string(), channel_id, count)),
    );
}

/// Game channels (players or spectators) the user has: (channel_type, channel_id, label, white_id, black_id, finished).
/// Intentionally includes only channels with persisted game-chat activity.
/// This keeps the Messages hub scoped to active conversations instead of all game memberships.
/// Visibility: players see game_players; spectators see game_spectators only if they've sent a message.
/// For finished games, players see one sidebar entry that defaults to game_players.
pub async fn get_game_channel_summaries_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<GameChannelSummary>, DbError> {
    let player_activity_rows = get_game_player_activity_for_user(conn, user_id).await?;
    let spectator_activity_rows = get_game_spectator_activity_for_user(conn, user_id).await?;

    if player_activity_rows.is_empty() && spectator_activity_rows.is_empty() {
        return Ok(Vec::new());
    }

    let player_ids: Vec<Uuid> = player_activity_rows
        .iter()
        .flat_map(|(_, _, white_id, black_id, _, _)| [*white_id, *black_id])
        .chain(
            spectator_activity_rows
                .iter()
                .flat_map(|(_, white_id, black_id, _, _)| [*white_id, *black_id]),
        )
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let usernames: Vec<(Uuid, String)> = users::table
        .filter(users::id.eq_any(&player_ids))
        .select((users::id, users::username))
        .load(conn)
        .await
        .map_err(DbError::from)?;
    let username_map: HashMap<Uuid, String> = usernames.into_iter().collect();

    let mut result = Vec::new();
    let mut finished_player_activity = HashMap::<String, (Uuid, Uuid, bool, DateTime<Utc>)>::new();
    for (channel_type, channel_id, white_id, black_id, finished, last_message_at) in
        player_activity_rows
    {
        let Some(last_message_at) = last_message_at else {
            continue;
        };

        if finished {
            finished_player_activity
                .entry(channel_id)
                .and_modify(|existing| {
                    if last_message_at > existing.3 {
                        *existing = (white_id, black_id, finished, last_message_at);
                    }
                })
                .or_insert((white_id, black_id, finished, last_message_at));
            continue;
        }

        if channel_type != shared_types::CHANNEL_TYPE_GAME_PLAYERS {
            continue;
        }

        let white_name = username_map
            .get(&white_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let black_name = username_map
            .get(&black_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        result.push(GameChannelSummary {
            channel_type: shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string(),
            channel_id,
            label: format!("{white_name} vs {black_name} (players)"),
            white_id,
            black_id,
            finished,
            last_message_at,
        });
    }

    for (channel_id, (white_id, black_id, finished, last_message_at)) in finished_player_activity {
        let white_name = username_map
            .get(&white_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let black_name = username_map
            .get(&black_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        result.push(GameChannelSummary {
            channel_type: shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string(),
            channel_id,
            label: format!("{white_name} vs {black_name}"),
            white_id,
            black_id,
            finished,
            last_message_at,
        });
    }

    for (channel_id, white_id, black_id, finished, last_message_at) in spectator_activity_rows {
        let Some(last_message_at) = last_message_at else {
            continue;
        };
        let white_name = username_map
            .get(&white_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        let black_name = username_map
            .get(&black_id)
            .cloned()
            .unwrap_or_else(|| "?".to_string());
        result.push(GameChannelSummary {
            channel_type: shared_types::CHANNEL_TYPE_GAME_SPECTATORS.to_string(),
            channel_id,
            label: format!("{white_name} vs {black_name} (spectators)"),
            white_id,
            black_id,
            finished,
            last_message_at,
        });
    }
    result.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));
    Ok(result)
}

async fn get_game_player_activity_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(String, String, Uuid, Uuid, bool, Option<DateTime<Utc>>)>, DbError> {
    chat_messages::table
        .inner_join(games::table.on(chat_messages::game_id.eq(games::id.nullable())))
        .filter(
            chat_messages::channel_type
                .eq(shared_types::CHANNEL_TYPE_GAME_PLAYERS)
                .or(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GAME_SPECTATORS)),
        )
        .filter(games::white_id.eq(user_id).or(games::black_id.eq(user_id)))
        .group_by((
            chat_messages::channel_type,
            chat_messages::channel_id,
            games::white_id,
            games::black_id,
            games::finished,
        ))
        .select((
            chat_messages::channel_type,
            chat_messages::channel_id,
            games::white_id,
            games::black_id,
            games::finished,
            max(chat_messages::created_at),
        ))
        .load(conn)
        .await
        .map_err(DbError::from)
}

async fn get_game_spectator_activity_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(String, Uuid, Uuid, bool, Option<DateTime<Utc>>)>, DbError> {
    chat_messages::table
        .inner_join(games::table.on(chat_messages::game_id.eq(games::id.nullable())))
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GAME_SPECTATORS))
        .filter(chat_messages::sender_id.eq(user_id))
        .filter(games::white_id.ne(user_id).and(games::black_id.ne(user_id)))
        .group_by((
            chat_messages::channel_id,
            games::white_id,
            games::black_id,
            games::finished,
        ))
        .select((
            chat_messages::channel_id,
            games::white_id,
            games::black_id,
            games::finished,
            max(chat_messages::created_at),
        ))
        .load(conn)
        .await
        .map_err(DbError::from)
}

pub async fn get_messages_hub_catalog_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<MessagesHubCatalog, DbError> {
    use crate::helpers::{get_blocked_user_ids, get_muted_tournament_nanoids};

    let blocked_user_ids = get_blocked_user_ids(conn, user_id).await?;
    let blocked_user_set: HashSet<Uuid> = blocked_user_ids.iter().copied().collect();

    let muted_tournament_nanoids = get_muted_tournament_nanoids(conn, user_id).await?;

    let dms = get_dm_conversation_summaries_for_user(conn, user_id, &blocked_user_set).await?;
    let tournaments =
        get_tournament_lobby_channel_summaries_for_user(conn, user_id, &muted_tournament_nanoids)
            .await?;
    let games = get_game_channel_summaries_for_user(conn, user_id).await?;

    Ok(MessagesHubCatalog {
        blocked_user_ids,
        dms,
        tournaments,
        games,
        has_global: true,
    })
}

pub async fn get_unread_counts_for_messages_hub_catalog(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    catalog: &MessagesHubCatalog,
) -> Result<Vec<(String, String, i64)>, DbError> {
    let receipt_channels: HashSet<(String, String)> = chat_read_receipts::table
        .filter(chat_read_receipts::user_id.eq(user_id))
        .select((
            chat_read_receipts::channel_type,
            chat_read_receipts::channel_id,
        ))
        .load(conn)
        .await
        .map_err(DbError::from)?
        .into_iter()
        .collect();
    let has_receipt = |channel_type: &str, channel_id: &str| {
        receipt_channels.contains(&(channel_type.to_string(), channel_id.to_string()))
    };

    let mut result = Vec::new();
    let channel_groups = [
        (
            shared_types::CHANNEL_TYPE_DIRECT,
            catalog
                .dms
                .iter()
                .map(|row| row.channel_id.clone())
                .collect::<Vec<_>>(),
        ),
        (
            shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY,
            catalog
                .tournaments
                .iter()
                .filter(|row| !row.muted)
                .map(|row| row.nanoid.clone())
                .collect::<Vec<_>>(),
        ),
        (
            shared_types::CHANNEL_TYPE_GAME_PLAYERS,
            catalog
                .games
                .iter()
                .filter(|row| row.channel_type == shared_types::CHANNEL_TYPE_GAME_PLAYERS)
                .map(|row| row.channel_id.clone())
                .collect::<Vec<_>>(),
        ),
        (
            shared_types::CHANNEL_TYPE_GLOBAL,
            if catalog.has_global {
                vec![shared_types::CHANNEL_TYPE_GLOBAL.to_string()]
            } else {
                Vec::new()
            },
        ),
    ];

    for (channel_type, channel_ids) in channel_groups {
        if channel_ids.is_empty() {
            continue;
        }
        let with_receipt = channel_ids
            .iter()
            .filter(|channel_id| has_receipt(channel_type, channel_id))
            .cloned()
            .collect::<Vec<_>>();
        let without_receipt = channel_ids
            .into_iter()
            .filter(|channel_id| !has_receipt(channel_type, channel_id))
            .collect::<Vec<_>>();

        let receipt_counts =
            unread_counts_for_receipt_channel_ids(conn, channel_type, &with_receipt, user_id)
                .await?;
        extend_unread_counts(&mut result, channel_type, receipt_counts);

        let missing_counts =
            unread_counts_for_channel_ids(conn, channel_type, &without_receipt, user_id).await?;
        extend_unread_counts(&mut result, channel_type, missing_counts);
    }
    Ok(result)
}

/// Unread count per channel for a user: (channel_type, channel_id, count).
/// Scoped to channels with persisted activity rather than all memberships.
pub async fn get_unread_counts_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(String, String, i64)>, DbError> {
    let catalog = get_messages_hub_catalog_for_user(conn, user_id).await?;
    get_unread_counts_for_messages_hub_catalog(conn, user_id, &catalog).await
}
