use crate::{
    db_error::DbError,
    models::{ChatMessage, Game, NewChatMessage, Tournament},
    schema::{chat_messages, chat_read_receipts, games, users},
    DbConn,
};
use chrono::Utc;
use diesel::{prelude::*, upsert::excluded};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

/// Canonical channel_id for a DM between two users (sorted UUIDs so both participants use the same key).
pub fn canonical_dm_channel_id(a: Uuid, b: Uuid) -> String {
    if a < b {
        format!("{}::{}", a, b)
    } else {
        format!("{}::{}", b, a)
    }
}

/// Insert a chat message and return the inserted row.
pub async fn insert_chat_message(
    conn: &mut DbConn<'_>,
    new: NewChatMessage<'_>,
) -> Result<ChatMessage, DbError> {
    new.insert(conn).await
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

/// Returns true if the global channel has at least one message.
pub async fn global_channel_has_messages(conn: &mut DbConn<'_>) -> Result<bool, DbError> {
    use diesel::dsl::count_star;
    let count: i64 = chat_messages::table
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GLOBAL))
        .filter(chat_messages::channel_id.eq(shared_types::CHANNEL_TYPE_GLOBAL))
        .select(count_star())
        .get_result(conn)
        .await
        .map_err(DbError::from)?;
    Ok(count > 0)
}

/// Returns true if the user is allowed to read from this chat channel.
/// Game chat: only players may ever read game_players; only spectators may read game_spectators
/// while the game is ongoing; when the game is finished, players may also read game_spectators.
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
            // Anyone can read tournament chat (e.g. view without joining). Send is restricted to participants elsewhere.
            Ok(Tournament::from_nanoid(channel_id, conn).await.is_ok())
        }
        shared_types::CHANNEL_TYPE_GAME_PLAYERS | shared_types::CHANNEL_TYPE_GAME_SPECTATORS => {
            let game = match Game::find_by_game_id(&shared_types::GameId(channel_id.to_string()), conn).await {
                Ok(g) => g,
                Err(_) => return Ok(false),
            };
            let is_player = user_id == game.white_id || user_id == game.black_id;

            if channel_type == shared_types::CHANNEL_TYPE_GAME_PLAYERS {
                // Spectators must never read player messages, even after the game is over.
                Ok(is_player)
            } else {
                // game_spectators: players may not read while game is ongoing; when finished, players may read.
                if game.finished {
                    Ok(true)
                } else {
                    Ok(!is_player)
                }
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
    diesel::insert_into(chat_read_receipts::table)
        .values((
            chat_read_receipts::user_id.eq(user_id),
            chat_read_receipts::channel_type.eq(channel_type),
            chat_read_receipts::channel_id.eq(channel_id),
            chat_read_receipts::last_read_at.eq(last_read_at),
        ))
        .on_conflict((
            chat_read_receipts::user_id,
            chat_read_receipts::channel_type,
            chat_read_receipts::channel_id,
        ))
        .do_update()
        .set(chat_read_receipts::last_read_at.eq(excluded(chat_read_receipts::last_read_at)))
        .execute(conn)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

/// Other user id and username for each DM conversation the user has (from chat_messages).
/// Excludes users the current user has blocked.
pub async fn get_dm_conversations_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(Uuid, String)>, DbError> {
    use crate::helpers::get_blocked_user_ids;
    use crate::schema::users;

    let channel_ids: Vec<String> = chat_messages::table
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_DIRECT))
        .select(chat_messages::channel_id)
        .distinct()
        .load(conn)
        .await
        .map_err(DbError::from)?;

    let mut other_ids: Vec<Uuid> = Vec::new();
    for id_str in channel_ids {
        if let Some(other) = other_user_from_dm_channel(&id_str, user_id) {
            other_ids.push(other);
        }
    }
    other_ids.sort();
    other_ids.dedup();

    let blocked = get_blocked_user_ids(conn, user_id).await?;
    let blocked: std::collections::HashSet<Uuid> = blocked.into_iter().collect();
    other_ids.retain(|id| !blocked.contains(id));

    if other_ids.is_empty() {
        return Ok(Vec::new());
    }

    let users_rows: Vec<(Uuid, String)> = users::table
        .filter(users::id.eq_any(&other_ids))
        .select((users::id, users::username))
        .load(conn)
        .await
        .map_err(DbError::from)?;

    Ok(users_rows)
}

fn other_user_from_dm_channel(channel_id: &str, me: Uuid) -> Option<Uuid> {
    let parts: Vec<&str> = channel_id.split("::").collect();
    if parts.len() != 2 {
        return None;
    }
    let a: Uuid = parts[0].parse().ok()?;
    let b: Uuid = parts[1].parse().ok()?;
    if a == me {
        Some(b)
    } else if b == me {
        Some(a)
    } else {
        None
    }
}

/// Tournament lobby channels the user has: (nanoid, tournament name).
/// Includes all tournaments the user has joined that have chat activity, including Finished.
pub async fn get_tournament_lobby_channels_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(String, String)>, DbError> {
    use crate::schema::{tournaments, tournaments_users};

    let user_tournament_nanoids: Vec<String> = tournaments_users::table
        .filter(tournaments_users::user_id.eq(user_id))
        .inner_join(tournaments::table.on(tournaments_users::tournament_id.eq(tournaments::id)))
        .select(tournaments::nanoid)
        .load(conn)
        .await
        .map_err(DbError::from)?;

    if user_tournament_nanoids.is_empty() {
        return Ok(Vec::new());
    }

    let channel_ids: Vec<String> = chat_messages::table
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY))
        .filter(chat_messages::channel_id.eq_any(&user_tournament_nanoids))
        .select(chat_messages::channel_id)
        .distinct()
        .load(conn)
        .await
        .map_err(DbError::from)?;

    if channel_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<(String, String)> = tournaments::table
        .filter(tournaments::nanoid.eq_any(&channel_ids))
        .select((tournaments::nanoid, tournaments::name))
        .load(conn)
        .await
        .map_err(DbError::from)?;

    Ok(rows)
}

/// True if the user is a participant in the tournament (player or organizer). Used to allow send in tournament chat.
pub async fn is_tournament_participant(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<bool, DbError> {
    let tournament = match Tournament::from_nanoid(tournament_nanoid, conn).await {
        Ok(t) => t,
        Err(_) => return Ok(false),
    };
    let is_player = tournament
        .players(conn)
        .await
        .map(|p| p.iter().any(|u| u.id == user_id))
        .unwrap_or(false);
    let is_organizer = tournament
        .organizers(conn)
        .await
        .map(|o| o.iter().any(|u| u.id == user_id))
        .unwrap_or(false);
    Ok(is_player || is_organizer)
}

/// Game channels (players or spectators) the user has: (channel_type, channel_id, label, white_id, black_id, finished).
/// Includes all games the user is part of (ongoing or finished), so notifications always have a conversation in the hub.
/// Visibility: players see game_players; spectators see game_spectators only if they've sent a message.
/// For finished games, players see one merged entry (game_players used as representative).
pub async fn get_game_channels_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(String, String, String, Uuid, Uuid, bool)>, DbError> {
    let channels: Vec<(String, String)> = chat_messages::table
        .filter(
            chat_messages::channel_type
                .eq(shared_types::CHANNEL_TYPE_GAME_PLAYERS)
                .or(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GAME_SPECTATORS)),
        )
        .select((chat_messages::channel_type, chat_messages::channel_id))
        .distinct()
        .load(conn)
        .await
        .map_err(DbError::from)?;

    let channel_ids: Vec<&str> = channels.iter().map(|(_, cid)| cid.as_str()).collect();
    if channel_ids.is_empty() {
        return Ok(Vec::new());
    }

    let game_rows: Vec<(String, Uuid, Uuid, bool)> = games::table
        .filter(games::nanoid.eq_any(&channel_ids))
        .select((games::nanoid, games::white_id, games::black_id, games::finished))
        .load(conn)
        .await
        .map_err(DbError::from)?;

    let game_map: std::collections::HashMap<String, (Uuid, Uuid, bool)> = game_rows
        .into_iter()
        .map(|(nanoid, w, b, finished)| (nanoid, (w, b, finished)))
        .collect();

    // Spectator channels where user has sent a message
    let spectator_channels_user_sent: std::collections::HashSet<(String, String)> =
        chat_messages::table
            .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GAME_SPECTATORS))
            .filter(chat_messages::sender_id.eq(user_id))
            .select((chat_messages::channel_type, chat_messages::channel_id))
            .distinct()
            .load::<(String, String)>(conn)
            .await
            .map_err(DbError::from)?
            .into_iter()
            .collect();

    let player_ids: Vec<Uuid> = game_map
        .values()
        .flat_map(|(w, b, _)| [*w, *b])
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let usernames: Vec<(Uuid, String)> = users::table
        .filter(users::id.eq_any(&player_ids))
        .select((users::id, users::username))
        .load(conn)
        .await
        .map_err(DbError::from)?;
    let username_map: std::collections::HashMap<Uuid, String> =
        usernames.into_iter().collect();

    let mut seen_finished_games: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result = Vec::new();
    for (channel_type, channel_id) in channels {
        if let Some((white_id, black_id, finished)) = game_map.get(&channel_id) {
            let is_player = user_id == *white_id || user_id == *black_id;
            let is_spectator = !is_player;

            let include_channel = if *finished {
                // For finished games, players see one merged entry (either channel type triggers inclusion).
                // Use game_players as the representative channel_type for consistency.
                if is_player && seen_finished_games.insert(channel_id.clone()) {
                    true
                } else {
                    false
                }
            } else if channel_type == shared_types::CHANNEL_TYPE_GAME_PLAYERS {
                is_player
            } else {
                is_spectator
                    && spectator_channels_user_sent.contains(&(channel_type.clone(), channel_id.clone()))
            };

            if include_channel {
                let white_name = username_map.get(white_id).cloned().unwrap_or_else(|| "?".to_string());
                let black_name = username_map.get(black_id).cloned().unwrap_or_else(|| "?".to_string());
                // For finished games, always use game_players as representative channel_type
                let effective_channel_type = if *finished {
                    shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string()
                } else {
                    channel_type.clone()
                };
                let label = if *finished {
                    format!("{} vs {}", white_name, black_name)
                } else if channel_type == shared_types::CHANNEL_TYPE_GAME_PLAYERS {
                    format!("{} vs {} (players)", white_name, black_name)
                } else {
                    format!("{} vs {} (spectators)", white_name, black_name)
                };
                result.push((
                    effective_channel_type,
                    channel_id.clone(),
                    label,
                    *white_id,
                    *black_id,
                    *finished,
                ));
            }
        }
    }
    Ok(result)
}

/// Unread count per channel for a user: (channel_type, channel_id, count).
/// Excludes muted tournament lobby channels and DMs with blocked users.
pub async fn get_unread_counts_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(String, String, i64)>, DbError> {
    use crate::helpers::{get_blocked_user_ids, get_muted_tournament_nanoids};
    use crate::schema::{games, tournaments, tournaments_users};

    let blocked_ids: std::collections::HashSet<Uuid> =
        get_blocked_user_ids(conn, user_id).await?.into_iter().collect();
    let muted_tournament_nanoids = get_muted_tournament_nanoids(conn, user_id).await?;

    let receipts: Vec<(String, String, chrono::DateTime<Utc>)> = chat_read_receipts::table
        .filter(chat_read_receipts::user_id.eq(user_id))
        .select((
            chat_read_receipts::channel_type,
            chat_read_receipts::channel_id,
            chat_read_receipts::last_read_at,
        ))
        .load(conn)
        .await
        .map_err(DbError::from)?;

    let mut result = Vec::with_capacity(receipts.len());
    let mut has_receipt: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for (channel_type, channel_id, last_read_at) in receipts {
        has_receipt.insert((channel_type.clone(), channel_id.clone()));
        // Only count messages from others, not the current user
        let count: i64 = chat_messages::table
            .filter(chat_messages::channel_type.eq(&channel_type))
            .filter(chat_messages::channel_id.eq(&channel_id))
            .filter(chat_messages::created_at.gt(last_read_at))
            .filter(chat_messages::sender_id.ne(user_id))
            .count()
            .get_result(conn)
            .await
            .map_err(DbError::from)?;
        if count > 0 {
            // Don't count unread for DMs with blocked users.
            if channel_type == shared_types::CHANNEL_TYPE_DIRECT {
                if let Some(other) = other_user_from_dm_channel(&channel_id, user_id) {
                    if blocked_ids.contains(&other) {
                        continue;
                    }
                }
            }
            result.push((channel_type, channel_id, count));
        }
    }

    // Channels user participates in but has no receipt: count all messages as unread.
    // Game channels: players see game_players (ongoing) or merged (finished); spectators see game_spectators if participated.
    let player_games: Vec<(String, bool)> = games::table
        .filter(games::white_id.eq(user_id).or(games::black_id.eq(user_id)))
        .select((games::nanoid, games::finished))
        .load(conn)
        .await
        .map_err(DbError::from)?;
    for (nanoid, finished) in player_games {
        if finished {
            let key = (shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string(), nanoid.clone());
            if has_receipt.contains(&key) {
                continue;
            }
            let c_players: i64 = chat_messages::table
                .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GAME_PLAYERS))
                .filter(chat_messages::channel_id.eq(&nanoid))
                .filter(chat_messages::sender_id.ne(user_id))
                .count()
                .get_result(conn)
                .await
                .map_err(DbError::from)?;
            let c_spectators: i64 = chat_messages::table
                .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GAME_SPECTATORS))
                .filter(chat_messages::channel_id.eq(&nanoid))
                .filter(chat_messages::sender_id.ne(user_id))
                .count()
                .get_result(conn)
                .await
                .map_err(DbError::from)?;
            let total = c_players + c_spectators;
            if total > 0 {
                result.push((shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string(), nanoid, total));
            }
        } else {
            let key = (shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string(), nanoid.clone());
            if has_receipt.contains(&key) {
                continue;
            }
            let c: i64 = chat_messages::table
                .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GAME_PLAYERS))
                .filter(chat_messages::channel_id.eq(&nanoid))
                .filter(chat_messages::sender_id.ne(user_id))
                .count()
                .get_result(conn)
                .await
                .map_err(DbError::from)?;
            if c > 0 {
                result.push((shared_types::CHANNEL_TYPE_GAME_PLAYERS.to_string(), nanoid, c));
            }
        }
    }

    // Spectator channels: games where user has sent a message (participated) and is not a player.
    let spectator_channels: Vec<(String, String)> = chat_messages::table
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_GAME_SPECTATORS))
        .filter(chat_messages::sender_id.eq(user_id))
        .select((chat_messages::channel_type, chat_messages::channel_id))
        .distinct()
        .load(conn)
        .await
        .map_err(DbError::from)?;
    for (channel_type, channel_id) in spectator_channels {
        let key = (channel_type.clone(), channel_id.clone());
        if has_receipt.contains(&key) {
            continue;
        }
        let game: Option<(Uuid, Uuid)> = games::table
            .filter(games::nanoid.eq(&channel_id))
            .select((games::white_id, games::black_id))
            .first(conn)
            .await
            .ok();
        if let Some((white_id, black_id)) = game {
            if user_id == white_id || user_id == black_id {
                continue;
            }
        }
        let c: i64 = chat_messages::table
            .filter(chat_messages::channel_type.eq(&channel_type))
            .filter(chat_messages::channel_id.eq(&channel_id))
            .filter(chat_messages::sender_id.ne(user_id))
            .count()
            .get_result(conn)
            .await
            .map_err(DbError::from)?;
        if c > 0 {
            result.push((channel_type, channel_id, c));
        }
    }

    // Tournament lobby channels: tournaments user is in.
    let tournament_nanoids: Vec<String> = tournaments_users::table
        .filter(tournaments_users::user_id.eq(user_id))
        .inner_join(tournaments::table.on(tournaments_users::tournament_id.eq(tournaments::id)))
        .select(tournaments::nanoid)
        .load(conn)
        .await
        .map_err(DbError::from)?;
    for nanoid in tournament_nanoids {
        if muted_tournament_nanoids.contains(&nanoid) {
            continue;
        }
        let key = (shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY.to_string(), nanoid.clone());
        if has_receipt.contains(&key) {
            continue;
        }
        let c: i64 = chat_messages::table
            .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY))
            .filter(chat_messages::channel_id.eq(&nanoid))
            .filter(chat_messages::sender_id.ne(user_id))
            .count()
            .get_result(conn)
            .await
            .map_err(DbError::from)?;
        if c > 0 {
            result.push((shared_types::CHANNEL_TYPE_TOURNAMENT_LOBBY.to_string(), nanoid, c));
        }
    }

    // Direct channels: DMs where user is a participant (channel_id = "uuid1::uuid2").
    let dm_channel_ids: Vec<String> = chat_messages::table
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_DIRECT))
        .select(chat_messages::channel_id)
        .distinct()
        .load(conn)
        .await
        .map_err(DbError::from)?;
    for channel_id in dm_channel_ids {
        let other = match other_user_from_dm_channel(&channel_id, user_id) {
            Some(o) => o,
            None => continue,
        };
        if blocked_ids.contains(&other) {
            continue;
        }
        let key = (shared_types::CHANNEL_TYPE_DIRECT.to_string(), channel_id.clone());
        if has_receipt.contains(&key) {
            continue;
        }
        let c: i64 = chat_messages::table
            .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_DIRECT))
            .filter(chat_messages::channel_id.eq(&channel_id))
            .filter(chat_messages::sender_id.ne(user_id))
            .count()
            .get_result(conn)
            .await
            .map_err(DbError::from)?;
        if c > 0 {
            result.push((shared_types::CHANNEL_TYPE_DIRECT.to_string(), channel_id, c));
        }
    }

    Ok(result)
}
