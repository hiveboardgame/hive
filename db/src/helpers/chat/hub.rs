use super::{user_display_map, UserDisplay, HUB_SECTION_LIMIT};
use crate::{db_error::DbError, models::User, DbConn};
use chrono::{DateTime, Utc};
use diesel::{
    prelude::*,
    sql_types::{Array, BigInt, Bool, Text, Timestamptz, Uuid as SqlUuid},
};
use diesel_async::RunQueryDsl;
use shared_types::{
    ConversationKey,
    ConversationUnreadState,
    DmConversation,
    GameChannel,
    GameChatCapabilities,
    GameId,
    TournamentChannel,
    TournamentChatCapabilities,
    TournamentId,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(QueryableByName, Clone, Debug)]
struct DmActivityRow {
    #[diesel(sql_type = SqlUuid)]
    other_user_id: Uuid,
    #[diesel(sql_type = Timestamptz)]
    last_message_at: DateTime<Utc>,
}

#[derive(QueryableByName, Clone, Debug)]
struct TournamentActivityRow {
    #[diesel(sql_type = Text)]
    nanoid: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Bool)]
    is_organizer: bool,
    #[diesel(sql_type = Bool)]
    is_participant: bool,
    #[diesel(sql_type = Timestamptz)]
    last_message_at: DateTime<Utc>,
}

#[derive(QueryableByName, Clone, Debug)]
struct GameActivityRow {
    #[diesel(sql_type = Text)]
    nanoid: String,
    #[diesel(sql_type = SqlUuid)]
    white_id: Uuid,
    #[diesel(sql_type = SqlUuid)]
    black_id: Uuid,
    #[diesel(sql_type = Bool)]
    finished: bool,
    #[diesel(sql_type = Timestamptz)]
    last_message_at: DateTime<Utc>,
}

#[derive(QueryableByName, Clone, Debug)]
struct DmUnreadStateRow {
    #[diesel(sql_type = SqlUuid)]
    other_user_id: Uuid,
    #[diesel(sql_type = BigInt)]
    unread_count: i64,
    #[diesel(sql_type = BigInt)]
    latest_message_id: i64,
    #[diesel(sql_type = BigInt)]
    latest_unread_message_id: i64,
    #[diesel(sql_type = BigInt)]
    last_read_message_id: i64,
}

#[derive(QueryableByName, Clone, Debug)]
struct TextUnreadStateRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = BigInt)]
    unread_count: i64,
    #[diesel(sql_type = BigInt)]
    latest_message_id: i64,
    #[diesel(sql_type = BigInt)]
    latest_unread_message_id: i64,
    #[diesel(sql_type = BigInt)]
    last_read_message_id: i64,
}

pub async fn unread_states_for_messages_hub_channels(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    dms: &[DmConversation],
    tournaments: &[TournamentChannel],
    games: &[GameChannel],
    muted_tournament_ids: &[TournamentId],
) -> Result<Vec<ConversationUnreadState>, DbError> {
    let mut states = Vec::new();
    states.extend(unread_dm_states(conn, user_id, dms).await?);
    states
        .extend(unread_tournament_states(conn, user_id, tournaments, muted_tournament_ids).await?);
    states.extend(unread_player_game_states(conn, user_id, games).await?);
    Ok(states)
}

async fn unread_dm_states(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    dms: &[DmConversation],
) -> Result<Vec<ConversationUnreadState>, DbError> {
    let peer_ids = dms.iter().map(|dm| dm.other_user_id).collect::<Vec<_>>();
    if peer_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<DmUnreadStateRow> = diesel::sql_query(
        r#"
        WITH peers AS (
            SELECT unnest($2) AS other_user_id
        )
        SELECT
            p.other_user_id,
            COUNT(cm.id) FILTER (
                WHERE cm.sender_id <> $1
                    AND cm.id > COALESCE(rr.last_read_message_id, 0)
            ) AS unread_count,
            COALESCE(MAX(cm.id), 0) AS latest_message_id,
            COALESCE(MAX(cm.id) FILTER (
                WHERE cm.sender_id <> $1
                    AND cm.id > COALESCE(rr.last_read_message_id, 0)
            ), 0) AS latest_unread_message_id,
            COALESCE(MAX(rr.last_read_message_id), 0) AS last_read_message_id
        FROM peers p
        LEFT JOIN chat_channels c
            ON c.kind = 'direct'
            AND c.direct_user_low_id = CASE WHEN $1 < p.other_user_id THEN $1 ELSE p.other_user_id END
            AND c.direct_user_high_id = CASE WHEN $1 < p.other_user_id THEN p.other_user_id ELSE $1 END
        LEFT JOIN chat_read_receipts rr
            ON rr.user_id = $1
            AND rr.channel_id = c.id
        LEFT JOIN chat_messages cm
            ON cm.channel_id = c.id
        GROUP BY p.other_user_id
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<Array<SqlUuid>, _>(peer_ids)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    Ok(rows
        .into_iter()
        .map(|row| ConversationUnreadState {
            key: ConversationKey::direct(row.other_user_id),
            count: row.unread_count,
            latest_message_id: row.latest_message_id,
            latest_unread_message_id: row.latest_unread_message_id,
            last_read_message_id: row.last_read_message_id.min(row.latest_message_id),
        })
        .collect())
}

async fn unread_tournament_states(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournaments: &[TournamentChannel],
    muted_tournament_ids: &[TournamentId],
) -> Result<Vec<ConversationUnreadState>, DbError> {
    let muted_tournament_ids = muted_tournament_ids.iter().collect::<HashSet<_>>();
    let tournament_ids = tournaments
        .iter()
        .map(|tournament| tournament.tournament_id.0.clone())
        .collect::<Vec<_>>();
    if tournament_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<TextUnreadStateRow> = diesel::sql_query(
        r#"
        WITH hub_tournaments AS (
            SELECT unnest($2) AS nanoid
        )
        SELECT
            ht.nanoid AS id,
            COUNT(cm.id) FILTER (
                WHERE cm.sender_id <> $1
                    AND cm.id > COALESCE(rr.last_read_message_id, 0)
            ) AS unread_count,
            COALESCE(MAX(cm.id), 0) AS latest_message_id,
            COALESCE(MAX(cm.id) FILTER (
                WHERE cm.sender_id <> $1
                    AND cm.id > COALESCE(rr.last_read_message_id, 0)
            ), 0) AS latest_unread_message_id,
            COALESCE(MAX(rr.last_read_message_id), 0) AS last_read_message_id
        FROM hub_tournaments ht
        JOIN tournaments t ON t.nanoid = ht.nanoid
        LEFT JOIN chat_channels c
            ON c.kind = 'tournament_lobby'
            AND c.tournament_id = t.id
        LEFT JOIN chat_read_receipts rr
            ON rr.user_id = $1
            AND rr.channel_id = c.id
        LEFT JOIN chat_messages cm
            ON cm.channel_id = c.id
        GROUP BY ht.nanoid
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<Array<Text>, _>(tournament_ids)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let tournament_id = TournamentId(row.id);
            let muted = muted_tournament_ids.contains(&tournament_id);
            ConversationUnreadState {
                key: ConversationKey::tournament(&tournament_id),
                count: if muted { 0 } else { row.unread_count },
                latest_message_id: row.latest_message_id,
                latest_unread_message_id: if muted {
                    0
                } else {
                    row.latest_unread_message_id
                },
                last_read_message_id: row.last_read_message_id.min(row.latest_message_id),
            }
        })
        .collect())
}

async fn unread_player_game_states(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    games: &[GameChannel],
) -> Result<Vec<ConversationUnreadState>, DbError> {
    let player_game_ids = games
        .iter()
        .map(|game| game.game_id.0.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if player_game_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows: Vec<TextUnreadStateRow> = diesel::sql_query(
        r#"
        WITH hub_games AS (
            SELECT unnest($2) AS nanoid
        )
        SELECT
            hg.nanoid AS id,
            COUNT(cm.id) FILTER (
                WHERE cm.sender_id <> $1
                    AND cm.id > COALESCE(rr.last_read_message_id, 0)
            ) AS unread_count,
            COALESCE(MAX(cm.id), 0) AS latest_message_id,
            COALESCE(MAX(cm.id) FILTER (
                WHERE cm.sender_id <> $1
                    AND cm.id > COALESCE(rr.last_read_message_id, 0)
            ), 0) AS latest_unread_message_id,
            COALESCE(MAX(rr.last_read_message_id), 0) AS last_read_message_id
        FROM hub_games hg
        JOIN games g ON g.nanoid = hg.nanoid
        LEFT JOIN chat_channels c
            ON c.kind = 'game_players'
            AND c.game_id = g.id
        LEFT JOIN chat_read_receipts rr
            ON rr.user_id = $1
            AND rr.channel_id = c.id
        LEFT JOIN chat_messages cm
            ON cm.channel_id = c.id
        GROUP BY hg.nanoid
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<Array<Text>, _>(player_game_ids)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    Ok(rows
        .into_iter()
        .map(|row| ConversationUnreadState {
            key: ConversationKey::game_players(&GameId(row.id)),
            count: row.unread_count,
            latest_message_id: row.latest_message_id,
            latest_unread_message_id: row.latest_unread_message_id,
            last_read_message_id: row.last_read_message_id.min(row.latest_message_id),
        })
        .collect())
}

pub async fn get_dm_conversations_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<DmConversation>, DbError> {
    let rows: Vec<DmActivityRow> = diesel::sql_query(
        r#"
        WITH activity AS (
            SELECT
                CASE
                    WHEN c.direct_user_low_id = $1 THEN c.direct_user_high_id
                    ELSE c.direct_user_low_id
                END AS other_user_id,
                MAX(cm.created_at) AS last_message_at,
                COUNT(cm.id) FILTER (
                    WHERE cm.sender_id <> $1
                        AND cm.id > COALESCE(rr.last_read_message_id, 0)
                ) AS unread_count
            FROM chat_channels c
            JOIN chat_messages cm ON cm.channel_id = c.id
            LEFT JOIN chat_read_receipts rr
                ON rr.user_id = $1
                AND rr.channel_id = c.id
            WHERE c.kind = 'direct'
                AND (c.direct_user_low_id = $1 OR c.direct_user_high_id = $1)
            GROUP BY c.id, c.direct_user_low_id, c.direct_user_high_id, rr.last_read_message_id
        ),
        ranked AS (
            SELECT
                other_user_id,
                last_message_at,
                unread_count,
                ROW_NUMBER() OVER (ORDER BY last_message_at DESC) AS activity_rank
            FROM activity
        )
        SELECT
            other_user_id,
            last_message_at
        FROM ranked
        WHERE unread_count > 0 OR activity_rank <= $2
        ORDER BY last_message_at DESC
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<BigInt, _>(HUB_SECTION_LIMIT)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    let users = user_display_map(conn, rows.iter().map(|row| row.other_user_id)).await?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            users.get(&row.other_user_id).map(|display| DmConversation {
                other_user_id: row.other_user_id,
                username: display.username.clone(),
                peer_deleted: display.deleted,
                last_message_at: row.last_message_at,
            })
        })
        .collect())
}

pub async fn get_tournament_channels_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<TournamentChannel>, DbError> {
    let is_site_admin = User::is_admin(&user_id, conn).await?;
    let rows: Vec<TournamentActivityRow> = diesel::sql_query(
        r#"
        WITH activity AS (
            SELECT
                t.id,
                t.nanoid,
                t.name,
                EXISTS (
                    SELECT 1 FROM tournaments_organizers o
                    WHERE o.tournament_id = t.id AND o.organizer_id = $1
                ) AS is_organizer,
                EXISTS (
                    SELECT 1 FROM tournaments_users tu
                    WHERE tu.tournament_id = t.id AND tu.user_id = $1
                ) AS is_participant,
                MAX(cm.created_at) AS last_message_at,
                COUNT(cm.id) FILTER (
                    WHERE cm.sender_id <> $1
                        AND cm.id > COALESCE(rr.last_read_message_id, 0)
                        AND NOT EXISTS (
                            SELECT 1 FROM user_tournament_chat_mutes mute
                            WHERE mute.tournament_id = t.id AND mute.user_id = $1
                        )
                ) AS unread_count
            FROM chat_channels c
            JOIN chat_messages cm ON cm.channel_id = c.id
            JOIN tournaments t ON t.id = c.tournament_id
            LEFT JOIN chat_read_receipts rr
                ON rr.user_id = $1
                AND rr.channel_id = c.id
            WHERE c.kind = 'tournament_lobby'
                AND (
                    EXISTS (
                        SELECT 1 FROM tournaments_organizers o
                        WHERE o.tournament_id = t.id AND o.organizer_id = $1
                    )
                    OR EXISTS (
                        SELECT 1 FROM tournaments_users tu
                        WHERE tu.tournament_id = t.id AND tu.user_id = $1
                    )
                )
            GROUP BY t.id, t.nanoid, t.name, rr.last_read_message_id
        ),
        ranked AS (
            SELECT
                nanoid,
                name,
                is_organizer,
                is_participant,
                last_message_at,
                unread_count,
                ROW_NUMBER() OVER (ORDER BY last_message_at DESC) AS activity_rank
            FROM activity
        )
        SELECT
            nanoid,
            name,
            is_organizer,
            is_participant,
            last_message_at
        FROM ranked
        WHERE unread_count > 0 OR activity_rank <= $2
        ORDER BY last_message_at DESC
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<BigInt, _>(HUB_SECTION_LIMIT)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    Ok(rows
        .into_iter()
        .map(|row| TournamentChannel {
            tournament_id: TournamentId(row.nanoid),
            name: row.name,
            access: TournamentChatCapabilities::new(
                is_site_admin,
                row.is_organizer,
                row.is_participant,
            ),
            last_message_at: row.last_message_at,
        })
        .collect())
}

pub async fn get_game_channels_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<GameChannel>, DbError> {
    let rows: Vec<GameActivityRow> = diesel::sql_query(
        r#"
        WITH player_activity AS (
            SELECT
                g.nanoid,
                g.white_id,
                g.black_id,
                g.finished,
                MAX(cm.created_at) AS last_message_at,
                COUNT(cm.id) FILTER (
                    WHERE cm.sender_id <> $1
                        AND cm.id > COALESCE(rr.last_read_message_id, 0)
                ) AS unread_count
            FROM chat_channels c
            JOIN chat_messages cm ON cm.channel_id = c.id
            JOIN games g ON g.id = c.game_id
            LEFT JOIN chat_read_receipts rr
                ON rr.user_id = $1
                AND rr.channel_id = c.id
            WHERE c.kind = 'game_players'
                AND (g.white_id = $1 OR g.black_id = $1)
            GROUP BY g.nanoid, g.white_id, g.black_id, g.finished, rr.last_read_message_id
        ),
        ranked AS (
            SELECT
                nanoid,
                white_id,
                black_id,
                finished,
                last_message_at,
                unread_count,
                ROW_NUMBER() OVER (ORDER BY last_message_at DESC) AS activity_rank
            FROM player_activity
        )
        SELECT
            nanoid,
            white_id,
            black_id,
            finished,
            last_message_at
        FROM ranked
        WHERE unread_count > 0 OR activity_rank <= $2
        ORDER BY last_message_at DESC
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<BigInt, _>(HUB_SECTION_LIMIT)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    let player_ids = rows
        .iter()
        .flat_map(|row| [row.white_id, row.black_id])
        .collect::<HashSet<_>>();
    let users = user_display_map(conn, player_ids).await?;
    let mut channels = rows
        .into_iter()
        .map(|row| {
            let is_player = row.white_id == user_id || row.black_id == user_id;
            game_channel_from_row(row, is_player, &users)
        })
        .collect::<Result<Vec<_>, _>>()?;
    channels.sort_by_key(|row| std::cmp::Reverse(row.last_message_at));
    Ok(channels)
}

fn game_channel_from_row(
    row: GameActivityRow,
    is_player: bool,
    users: &HashMap<Uuid, UserDisplay>,
) -> Result<GameChannel, DbError> {
    let white = users
        .get(&row.white_id)
        .ok_or_else(|| DbError::NotFound {
            reason: format!(
                "Missing white player {} for game {}",
                row.white_id, row.nanoid
            ),
        })?
        .display_name();
    let black = users
        .get(&row.black_id)
        .ok_or_else(|| DbError::NotFound {
            reason: format!(
                "Missing black player {} for game {}",
                row.black_id, row.nanoid
            ),
        })?
        .display_name();
    Ok(GameChannel {
        game_id: GameId(row.nanoid),
        label: format!("{white} vs {black}"),
        access: GameChatCapabilities::new(is_player, row.finished),
        last_message_at: row.last_message_at,
    })
}
