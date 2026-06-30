use crate::{
    db_error::DbError,
    models::{ChatMessage, NewChatMessage, NewChatReadReceipt, User},
    schema::{
        chat_messages,
        chat_read_receipts,
        games,
        tournaments,
        tournaments_organizers,
        tournaments_users,
        user_tournament_chat_mutes,
        users,
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{
    dsl::{exists, max, sql},
    prelude::*,
    sql_types::Timestamptz,
};
use diesel_async::RunQueryDsl;
use shared_types::{
    ChannelType,
    ConversationKey,
    DmConversation,
    GameChannel,
    GameChatCapabilities,
    GameId,
    GameThread,
    PersistentChannelKey,
    TournamentChannel,
    TournamentChatCapabilities,
    TournamentId,
    UnreadCount,
    CHANNEL_TYPE_DIRECT,
    CHANNEL_TYPE_GAME_PLAYERS,
    CHANNEL_TYPE_TOURNAMENT_LOBBY,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

diesel::allow_columns_to_appear_in_same_group_by_clause!(
    chat_messages::channel_type,
    chat_messages::channel_id,
    games::white_id,
    games::black_id,
    games::finished,
);

diesel::allow_columns_to_appear_in_same_group_by_clause!(
    tournaments::id,
    tournaments::nanoid,
    tournaments::name,
);

pub async fn get_game_chat_participants_and_finished(
    conn: &mut DbConn<'_>,
    game_id: &GameId,
) -> Result<(Uuid, Uuid, bool), DbError> {
    games::table
        .filter(games::nanoid.eq(&game_id.0))
        .select((games::white_id, games::black_id, games::finished))
        .first::<(Uuid, Uuid, bool)>(conn)
        .await
        .map_err(DbError::from)
}

pub async fn get_tournament_thread_data(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<(String, bool, TournamentChatCapabilities), DbError> {
    let is_site_admin = User::is_admin(&user_id, conn).await?;
    let (name, muted, is_organizer, is_participant) = tournaments::table
        .filter(tournaments::nanoid.eq(tournament_nanoid))
        .select((
            tournaments::name,
            exists(
                user_tournament_chat_mutes::table
                    .filter(user_tournament_chat_mutes::user_id.eq(user_id))
                    .filter(user_tournament_chat_mutes::tournament_id.eq(tournaments::id)),
            ),
            exists(
                tournaments_organizers::table
                    .filter(tournaments_organizers::organizer_id.eq(user_id))
                    .filter(tournaments_organizers::tournament_id.eq(tournaments::id)),
            ),
            exists(
                tournaments_users::table
                    .filter(tournaments_users::user_id.eq(user_id))
                    .filter(tournaments_users::tournament_id.eq(tournaments::id)),
            ),
        ))
        .first(conn)
        .await
        .map_err(DbError::from)?;
    Ok((
        name,
        muted,
        TournamentChatCapabilities::new(is_site_admin, is_organizer, is_participant),
    ))
}

pub async fn get_tournament_chat_capabilities(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<TournamentChatCapabilities, DbError> {
    let is_site_admin = User::is_admin(&user_id, conn).await?;
    let (is_organizer, is_participant) = tournaments::table
        .filter(tournaments::nanoid.eq(tournament_nanoid))
        .select((
            exists(
                tournaments_organizers::table
                    .filter(tournaments_organizers::organizer_id.eq(user_id))
                    .filter(tournaments_organizers::tournament_id.eq(tournaments::id)),
            ),
            exists(
                tournaments_users::table
                    .filter(tournaments_users::user_id.eq(user_id))
                    .filter(tournaments_users::tournament_id.eq(tournaments::id)),
            ),
        ))
        .first(conn)
        .await
        .map_err(DbError::from)?;
    Ok(TournamentChatCapabilities::new(
        is_site_admin,
        is_organizer,
        is_participant,
    ))
}

/// Insert a chat message and return the inserted row.
pub async fn insert_chat_message(
    conn: &mut DbConn<'_>,
    new: NewChatMessage<'_>,
) -> Result<ChatMessage, DbError> {
    let channel_key =
        PersistentChannelKey::from_raw(new.channel_type, new.channel_id).ok_or_else(|| {
            DbError::InvalidInput {
                info: "Invalid chat channel".to_string(),
                error: format!("{}:{}", new.channel_type, new.channel_id),
            }
        })?;
    let game_id = match new.game_id {
        Some(game_id) => Some(game_id),
        None => resolve_game_id_for_channel(conn, &channel_key).await?,
    };
    ensure_game_channel_has_game_id(&channel_key, &game_id)?;
    NewChatMessage { game_id, ..new }.insert(conn).await
}

/// Load messages for a channel, newest first.
pub async fn get_chat_messages_for_channel(
    conn: &mut DbConn<'_>,
    channel_key: &PersistentChannelKey,
    limit: i64,
) -> Result<Vec<ChatMessage>, DbError> {
    chat_messages::table
        .filter(chat_messages::channel_type.eq(channel_key.channel_type.as_str()))
        .filter(chat_messages::channel_id.eq(&channel_key.channel_id))
        .order(chat_messages::created_at.desc())
        .then_order_by(chat_messages::id.desc())
        .limit(limit)
        .get_results(conn)
        .await
        .map_err(DbError::from)
}

fn is_game_chat_channel_type(channel_type: ChannelType) -> bool {
    matches!(
        channel_type,
        ChannelType::GamePlayers | ChannelType::GameSpectators
    )
}

fn ensure_game_channel_has_game_id(
    channel_key: &PersistentChannelKey,
    game_id: &Option<Uuid>,
) -> Result<(), DbError> {
    if is_game_chat_channel_type(channel_key.channel_type) && game_id.is_none() {
        return Err(DbError::NotFound {
            reason: format!("Game chat channel not found: {}", channel_key.channel_id),
        });
    }
    Ok(())
}

async fn resolve_game_id_for_channel(
    conn: &mut DbConn<'_>,
    channel_key: &PersistentChannelKey,
) -> Result<Option<Uuid>, DbError> {
    if !is_game_chat_channel_type(channel_key.channel_type) {
        return Ok(None);
    }

    match games::table
        .filter(games::nanoid.eq(&channel_key.channel_id))
        .select(games::id)
        .first(conn)
        .await
    {
        Ok(game_id) => Ok(Some(game_id)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(err) => Err(DbError::from(err)),
    }
}

/// Upsert a read receipt: set last_read_at for (user_id, channel_type, channel_id).
pub async fn upsert_chat_read_receipt(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    channel_key: &PersistentChannelKey,
    last_read_at: chrono::DateTime<Utc>,
) -> Result<(), DbError> {
    let game_id = resolve_game_id_for_channel(conn, channel_key).await?;
    ensure_game_channel_has_game_id(channel_key, &game_id)?;
    let new_receipt = NewChatReadReceipt {
        user_id,
        channel_type: channel_key.channel_type.as_str(),
        channel_id: &channel_key.channel_id,
        last_read_at,
        game_id,
    };

    diesel::insert_into(chat_read_receipts::table)
        .values(&new_receipt)
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
            chat_read_receipts::game_id.eq(new_receipt.game_id),
        ))
        .execute(conn)
        .await
        .map_err(DbError::from)?;
    Ok(())
}

/// Other user id and username for each DM conversation the user has (from chat_messages).
/// Includes the latest persisted activity timestamp.
pub async fn get_dm_conversations_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<DmConversation>, DbError> {
    let mut other_id_to_activity = HashMap::<Uuid, DateTime<Utc>>::new();

    for (other_user_id, last_message_at) in get_sent_dm_activity_for_user(conn, user_id).await? {
        let (Some(other_user_id), Some(last_message_at)) = (other_user_id, last_message_at) else {
            continue;
        };
        update_latest_dm_activity(&mut other_id_to_activity, other_user_id, last_message_at);
    }

    for (other_user_id, last_message_at) in get_received_dm_activity_for_user(conn, user_id).await?
    {
        let Some(last_message_at) = last_message_at else {
            continue;
        };
        update_latest_dm_activity(&mut other_id_to_activity, other_user_id, last_message_at);
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
        .filter_map(|(other_user_id, last_message_at)| {
            username_map
                .get(&other_user_id)
                .cloned()
                .map(|username| DmConversation {
                    other_user_id,
                    username,
                    last_message_at,
                })
        })
        .collect::<Vec<_>>();
    result.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));

    Ok(result)
}

fn update_latest_dm_activity(
    activity_by_other_user: &mut HashMap<Uuid, DateTime<Utc>>,
    other_user_id: Uuid,
    last_message_at: DateTime<Utc>,
) {
    activity_by_other_user
        .entry(other_user_id)
        .and_modify(|existing| {
            if last_message_at > *existing {
                *existing = last_message_at;
            }
        })
        .or_insert(last_message_at);
}

async fn get_sent_dm_activity_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(Option<Uuid>, Option<DateTime<Utc>>)>, DbError> {
    chat_messages::table
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_DIRECT))
        .filter(chat_messages::sender_id.eq(user_id))
        .group_by(chat_messages::recipient_id)
        .select((chat_messages::recipient_id, max(chat_messages::created_at)))
        .load(conn)
        .await
        .map_err(DbError::from)
}

async fn get_received_dm_activity_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<(Uuid, Option<DateTime<Utc>>)>, DbError> {
    chat_messages::table
        .filter(chat_messages::channel_type.eq(shared_types::CHANNEL_TYPE_DIRECT))
        .filter(chat_messages::recipient_id.eq(Some(user_id)))
        .group_by(chat_messages::sender_id)
        .select((chat_messages::sender_id, max(chat_messages::created_at)))
        .load(conn)
        .await
        .map_err(DbError::from)
}

pub async fn get_tournament_channels_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<TournamentChannel>, DbError> {
    let is_site_admin = User::is_admin(&user_id, conn).await?;
    let mut result = chat_messages::table
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
        .group_by((tournaments::id, tournaments::nanoid, tournaments::name))
        .select((
            tournaments::nanoid,
            tournaments::name,
            exists(
                tournaments_organizers::table
                    .filter(tournaments_organizers::organizer_id.eq(user_id))
                    .filter(tournaments_organizers::tournament_id.eq(tournaments::id)),
            ),
            exists(
                tournaments_users::table
                    .filter(tournaments_users::user_id.eq(user_id))
                    .filter(tournaments_users::tournament_id.eq(tournaments::id)),
            ),
            max(chat_messages::created_at),
        ))
        .load::<(String, String, bool, bool, Option<DateTime<Utc>>)>(conn)
        .await
        .map_err(DbError::from)?
        .into_iter()
        .filter_map(
            |(nanoid, name, is_organizer, is_participant, last_message_at)| {
                last_message_at.map(|last_message_at| TournamentChannel {
                    tournament_id: TournamentId(nanoid),
                    name,
                    access: TournamentChatCapabilities::new(
                        is_site_admin,
                        is_organizer,
                        is_participant,
                    ),
                    last_message_at,
                })
            },
        )
        .collect::<Vec<_>>();
    result.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));
    Ok(result)
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
        .left_join(
            chat_read_receipts::table.on(chat_messages::channel_type
                .eq(chat_read_receipts::channel_type)
                .and(chat_messages::channel_id.eq(chat_read_receipts::channel_id))
                .and(chat_read_receipts::user_id.eq(user_id))),
        )
        .filter(chat_messages::channel_type.eq(channel_type))
        .filter(chat_messages::channel_id.eq_any(channel_ids))
        .filter(chat_messages::sender_id.ne(user_id))
        // Unread = newer than the receipt. `created_at` (send-time clock) and
        // `last_read_at` (read-time clock) are written by separate, untransacted
        // statements, so a send racing a mark-read can leave a phantom +1 here.
        // Accepted: it self-heals on the next mark-read. A race-free fix would
        // track last-read by monotonic message id instead of timestamp.
        .filter(
            chat_read_receipts::last_read_at
                .is_null()
                .or(chat_messages::created_at.gt(chat_read_receipts::last_read_at)),
        )
        .group_by(chat_messages::channel_id)
        .select((chat_messages::channel_id, count_star()))
        .load(conn)
        .await
        .map_err(DbError::from)
}

fn extend_unread_counts(
    result: &mut Vec<UnreadCount>,
    channel_map: &HashMap<String, ConversationKey>,
    unread_counts: Vec<(String, i64)>,
) {
    result.extend(unread_counts.into_iter().filter_map(|(channel_id, count)| {
        channel_map
            .get(&channel_id)
            .cloned()
            .map(|key| UnreadCount { key, count })
    }));
}

/// Game channels (players or spectators) the user has.
/// Intentionally includes only channels with persisted game-chat activity.
/// This keeps the Messages hub scoped to active conversations instead of all game memberships.
/// Visibility: players see game_players; spectators see game_spectators only if they've sent a message.
/// For finished games, players see one sidebar entry that defaults to game_players.
pub async fn get_game_channels_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<GameChannel>, DbError> {
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

    let username = |id| {
        username_map
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "?".to_string())
    };
    let build_game_channel = |channel_id: String,
                              thread: GameThread,
                              activity: (Uuid, Uuid, DateTime<Utc>),
                              finished| {
        let (white_id, black_id, last_message_at) = activity;

        GameChannel {
            game_id: GameId(channel_id),
            thread,
            label: format!("{} vs {}", username(white_id), username(black_id)),
            access: GameChatCapabilities::new(white_id == user_id || black_id == user_id, finished),
            last_message_at,
        }
    };

    let mut result = Vec::new();
    let mut finished_player_activity = HashMap::<String, (Uuid, Uuid, DateTime<Utc>)>::new();
    for (channel_type, channel_id, white_id, black_id, finished, last_message_at) in
        player_activity_rows
    {
        let Some(last_message_at) = last_message_at else {
            continue;
        };
        let activity = (white_id, black_id, last_message_at);

        if finished {
            finished_player_activity
                .entry(channel_id)
                .and_modify(|existing| {
                    if activity.2 > existing.2 {
                        *existing = activity;
                    }
                })
                .or_insert(activity);
            continue;
        }

        if channel_type != shared_types::CHANNEL_TYPE_GAME_PLAYERS {
            continue;
        }

        result.push(build_game_channel(
            channel_id,
            GameThread::Players,
            activity,
            false,
        ));
    }

    for (channel_id, activity) in finished_player_activity {
        result.push(build_game_channel(
            channel_id,
            GameThread::Players,
            activity,
            true,
        ));
    }

    for (channel_id, white_id, black_id, finished, last_message_at) in spectator_activity_rows {
        let Some(last_message_at) = last_message_at else {
            continue;
        };
        let activity = (white_id, black_id, last_message_at);
        result.push(build_game_channel(
            channel_id,
            GameThread::Spectators,
            activity,
            finished,
        ));
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

pub async fn get_unread_counts_for_messages_hub_channels(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    dms: &[DmConversation],
    tournaments: &[TournamentChannel],
    games: &[GameChannel],
    muted_tournament_ids: &[TournamentId],
) -> Result<Vec<UnreadCount>, DbError> {
    let mut result = Vec::new();
    let muted_tournament_ids = muted_tournament_ids.iter().collect::<HashSet<_>>();
    // Global announcements stay readable in /messages, but they do not participate in unread badges.
    let channel_groups = [
        (
            CHANNEL_TYPE_DIRECT,
            dms.iter()
                .map(|row| {
                    let key = ConversationKey::direct(row.other_user_id);
                    let persistent_key = PersistentChannelKey::direct(user_id, row.other_user_id);
                    (key, persistent_key)
                })
                .collect::<Vec<_>>(),
        ),
        (
            CHANNEL_TYPE_TOURNAMENT_LOBBY,
            tournaments
                .iter()
                .filter(|row| !muted_tournament_ids.contains(&row.tournament_id))
                .map(|row| {
                    let key = ConversationKey::tournament(&row.tournament_id);
                    let persistent_key = PersistentChannelKey::tournament(&row.tournament_id);
                    (key, persistent_key)
                })
                .collect::<Vec<_>>(),
        ),
        (
            CHANNEL_TYPE_GAME_PLAYERS,
            games
                .iter()
                .filter(|row| row.thread == GameThread::Players)
                .map(|row| {
                    let key = ConversationKey::game_players(&row.game_id);
                    let persistent_key = PersistentChannelKey::game_players(&row.game_id);
                    (key, persistent_key)
                })
                .collect::<Vec<_>>(),
        ),
    ];

    for (channel_type, channels) in channel_groups {
        let channel_map: HashMap<String, ConversationKey> = channels
            .iter()
            .map(|(key, persistent_key)| (persistent_key.channel_id.clone(), key.clone()))
            .collect();
        let channel_ids = channels
            .into_iter()
            .map(|(_, persistent_key)| persistent_key.channel_id)
            .collect::<Vec<_>>();

        let unread_counts =
            unread_counts_for_channel_ids(conn, channel_type, &channel_ids, user_id).await?;
        extend_unread_counts(&mut result, &channel_map, unread_counts);
    }
    Ok(result)
}
