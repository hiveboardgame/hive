use super::HUB_SECTION_LIMIT;
use crate::{db_error::DbError, DbConn};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Bool, Text, Uuid as SqlUuid},
};
use diesel_async::RunQueryDsl;
use shared_types::{
    ConversationKey,
    ConversationUnreadState,
    DmConversation,
    GameChannel,
    GameId,
    TournamentChannel,
    TournamentId,
};
use uuid::Uuid;

#[derive(QueryableByName, Clone, Debug)]
struct DmActivityRow {
    #[diesel(sql_type = SqlUuid)]
    other_user_id: Uuid,
    #[diesel(sql_type = Text)]
    username: String,
    #[diesel(sql_type = Bool)]
    peer_deleted: bool,
    #[diesel(sql_type = BigInt)]
    last_message_id: i64,
}

#[derive(QueryableByName, Clone, Debug)]
struct TournamentActivityRow {
    #[diesel(sql_type = Text)]
    nanoid: String,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = BigInt)]
    last_message_id: i64,
}

#[derive(QueryableByName, Clone, Debug)]
struct GameActivityRow {
    #[diesel(sql_type = Text)]
    nanoid: String,
    #[diesel(sql_type = Text)]
    white_name: String,
    #[diesel(sql_type = Text)]
    black_name: String,
    #[diesel(sql_type = Bool)]
    finished: bool,
    #[diesel(sql_type = BigInt)]
    last_message_id: i64,
}

#[derive(QueryableByName, Clone, Debug)]
struct InboxUnreadStateRow {
    #[diesel(sql_type = Text)]
    channel_kind: String,
    #[diesel(sql_type = Text)]
    key_id: String,
    #[diesel(sql_type = BigInt)]
    unread_count: i64,
    #[diesel(sql_type = BigInt)]
    latest_message_id: i64,
    #[diesel(sql_type = BigInt)]
    latest_unread_message_id: i64,
    #[diesel(sql_type = BigInt)]
    last_read_message_id: i64,
}

pub async fn chat_inbox_unread_states(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<ConversationUnreadState>, DbError> {
    let rows: Vec<InboxUnreadStateRow> = diesel::sql_query(
        r#"
        WITH inbox_channels AS (
            SELECT
                c.id AS channel_id,
                'direct'::text AS channel_kind,
                (CASE
                    WHEN c.direct_user_low_id = $1 THEN c.direct_user_high_id
                    ELSE c.direct_user_low_id
                END)::text AS key_id
            FROM chat_channels c
            WHERE c.kind = 'direct'
                AND (c.direct_user_low_id = $1 OR c.direct_user_high_id = $1)

            UNION ALL

            SELECT
                c.id AS channel_id,
                'tournament'::text AS channel_kind,
                t.nanoid AS key_id
            FROM chat_channels c
            JOIN tournaments t ON t.id = c.tournament_id
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
                AND NOT EXISTS (
                    SELECT 1 FROM user_tournament_chat_mutes mute
                    WHERE mute.tournament_id = t.id AND mute.user_id = $1
                )

            UNION ALL

            SELECT
                c.id AS channel_id,
                'game_players'::text AS channel_kind,
                g.nanoid AS key_id
            FROM chat_channels c
            JOIN games g ON g.id = c.game_id
            WHERE c.kind = 'game_players'
                AND (g.white_id = $1 OR g.black_id = $1)
        )
        SELECT
            inbox.channel_kind,
            inbox.key_id,
            COALESCE(unread.unread_count, 0) AS unread_count,
            COALESCE(latest.id, 0) AS latest_message_id,
            COALESCE(unread.latest_unread_message_id, 0) AS latest_unread_message_id,
            COALESCE(rr.last_read_message_id, 0) AS last_read_message_id
        FROM inbox_channels inbox
        LEFT JOIN chat_read_receipts rr
            ON rr.user_id = $1
            AND rr.channel_id = inbox.channel_id
        LEFT JOIN LATERAL (
            SELECT cm.id
            FROM chat_messages cm
            WHERE cm.channel_id = inbox.channel_id
            ORDER BY cm.id DESC
            LIMIT 1
        ) latest ON true
        LEFT JOIN LATERAL (
            SELECT
                COUNT(*) AS unread_count,
                MAX(cm.id) AS latest_unread_message_id
            FROM chat_messages cm
            WHERE cm.channel_id = inbox.channel_id
                AND cm.id > COALESCE(rr.last_read_message_id, 0)
                AND cm.sender_id <> $1
        ) unread ON true
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    rows.into_iter()
        .map(|row| {
            let key = match row.channel_kind.as_str() {
                "direct" => {
                    ConversationKey::direct(row.key_id.parse::<Uuid>().map_err(|error| {
                        DbError::InvalidInput {
                            info: "Invalid direct-message inbox key".to_string(),
                            error: error.to_string(),
                        }
                    })?)
                }
                "tournament" => ConversationKey::tournament(&TournamentId(row.key_id)),
                "game_players" => ConversationKey::game_players(&GameId(row.key_id)),
                kind => {
                    return Err(DbError::InvalidInput {
                        info: "Invalid chat inbox channel kind".to_string(),
                        error: kind.to_string(),
                    });
                }
            };
            Ok(ConversationUnreadState {
                key,
                count: row.unread_count,
                latest_message_id: row.latest_message_id,
                latest_unread_message_id: row.latest_unread_message_id,
                last_read_message_id: row.last_read_message_id.min(row.latest_message_id),
            })
        })
        .collect::<Result<Vec<_>, DbError>>()
}

pub async fn get_dm_conversations_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<DmConversation>, DbError> {
    let rows: Vec<DmActivityRow> = diesel::sql_query(
        r#"
        WITH eligible AS (
            SELECT
                c.id AS channel_id,
                CASE
                    WHEN c.direct_user_low_id = $1 THEN c.direct_user_high_id
                    ELSE c.direct_user_low_id
                END AS other_user_id
            FROM chat_channels c
            WHERE c.kind = 'direct'
                AND (c.direct_user_low_id = $1 OR c.direct_user_high_id = $1)
        ),
        activity AS MATERIALIZED (
            SELECT
                eligible.channel_id,
                eligible.other_user_id,
                latest.id AS last_message_id
            FROM eligible
            JOIN LATERAL (
                SELECT cm.id
                FROM chat_messages cm
                WHERE cm.channel_id = eligible.channel_id
                ORDER BY cm.id DESC
                LIMIT 1
            ) latest ON true
        ),
        recent AS (
            SELECT activity.*
            FROM activity
            ORDER BY activity.last_message_id DESC
            LIMIT $2
        ),
        unread AS (
            SELECT activity.*
            FROM activity
            LEFT JOIN chat_read_receipts rr
                ON rr.user_id = $1
                AND rr.channel_id = activity.channel_id
            WHERE EXISTS (
                SELECT 1
                FROM chat_messages message
                WHERE message.channel_id = activity.channel_id
                    AND message.id > COALESCE(rr.last_read_message_id, 0)
                    AND message.sender_id <> $1
            )
        ),
        selected AS (
            SELECT * FROM recent
            UNION
            SELECT * FROM unread
        )
        SELECT
            selected.other_user_id,
            peer.username,
            peer.deleted AS peer_deleted,
            selected.last_message_id
        FROM selected
        JOIN users peer ON peer.id = selected.other_user_id
        ORDER BY selected.last_message_id DESC
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<BigInt, _>(HUB_SECTION_LIMIT)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    Ok(rows
        .into_iter()
        .map(|row| DmConversation {
            other_user_id: row.other_user_id,
            username: row.username,
            peer_deleted: row.peer_deleted,
            last_message_id: row.last_message_id,
        })
        .collect())
}

pub async fn get_tournament_channels_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<TournamentChannel>, DbError> {
    let rows: Vec<TournamentActivityRow> = diesel::sql_query(
        r#"
        WITH accessible_tournaments AS (
            SELECT o.tournament_id
            FROM tournaments_organizers o
            WHERE o.organizer_id = $1

            UNION

            SELECT tu.tournament_id
            FROM tournaments_users tu
            WHERE tu.user_id = $1
        ),
        eligible AS (
            SELECT
                c.id AS channel_id,
                c.tournament_id
            FROM accessible_tournaments access
            JOIN chat_channels c
                ON c.tournament_id = access.tournament_id
                AND c.kind = 'tournament_lobby'
        ),
        activity AS MATERIALIZED (
            SELECT
                eligible.channel_id,
                eligible.tournament_id,
                latest.id AS last_message_id
            FROM eligible
            JOIN LATERAL (
                SELECT cm.id
                FROM chat_messages cm
                WHERE cm.channel_id = eligible.channel_id
                ORDER BY cm.id DESC
                LIMIT 1
            ) latest ON true
        ),
        recent AS (
            SELECT activity.*
            FROM activity
            ORDER BY activity.last_message_id DESC
            LIMIT $2
        ),
        unread AS (
            SELECT activity.*
            FROM activity
            LEFT JOIN chat_read_receipts rr
                ON rr.user_id = $1
                AND rr.channel_id = activity.channel_id
            WHERE NOT EXISTS (
                SELECT 1
                FROM user_tournament_chat_mutes mute
                WHERE mute.user_id = $1
                    AND mute.tournament_id = activity.tournament_id
            )
                AND EXISTS (
                    SELECT 1
                    FROM chat_messages message
                    WHERE message.channel_id = activity.channel_id
                        AND message.id > COALESCE(rr.last_read_message_id, 0)
                        AND message.sender_id <> $1
                )
        ),
        selected AS (
            SELECT * FROM recent
            UNION
            SELECT * FROM unread
        )
        SELECT
            tournament.nanoid,
            tournament.name,
            selected.last_message_id
        FROM selected
        JOIN tournaments tournament ON tournament.id = selected.tournament_id
        ORDER BY selected.last_message_id DESC
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
            last_message_id: row.last_message_id,
        })
        .collect())
}

pub async fn get_game_channels_for_user(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
) -> Result<Vec<GameChannel>, DbError> {
    let rows: Vec<GameActivityRow> = diesel::sql_query(
        r#"
        WITH eligible_games AS (
            SELECT g.id AS game_id
            FROM games g
            WHERE g.white_id = $1

            UNION

            SELECT g.id AS game_id
            FROM games g
            WHERE g.black_id = $1
        ),
        eligible AS (
            SELECT
                c.id AS channel_id,
                c.game_id
            FROM eligible_games game
            JOIN chat_channels c
                ON c.game_id = game.game_id
                AND c.kind = 'game_players'
        ),
        activity AS MATERIALIZED (
            SELECT
                eligible.channel_id,
                eligible.game_id,
                latest.id AS last_message_id
            FROM eligible
            JOIN LATERAL (
                SELECT cm.id
                FROM chat_messages cm
                WHERE cm.channel_id = eligible.channel_id
                ORDER BY cm.id DESC
                LIMIT 1
            ) latest ON true
        ),
        recent AS (
            SELECT activity.*
            FROM activity
            ORDER BY activity.last_message_id DESC
            LIMIT $2
        ),
        unread AS (
            SELECT activity.*
            FROM activity
            LEFT JOIN chat_read_receipts rr
                ON rr.user_id = $1
                AND rr.channel_id = activity.channel_id
            WHERE EXISTS (
                SELECT 1
                FROM chat_messages message
                WHERE message.channel_id = activity.channel_id
                    AND message.id > COALESCE(rr.last_read_message_id, 0)
                    AND message.sender_id <> $1
            )
        ),
        selected AS (
            SELECT * FROM recent
            UNION
            SELECT * FROM unread
        )
        SELECT
            game.nanoid,
            CASE WHEN white.deleted THEN 'Deleted user' ELSE white.username END AS white_name,
            CASE WHEN black.deleted THEN 'Deleted user' ELSE black.username END AS black_name,
            game.finished,
            selected.last_message_id
        FROM selected
        JOIN games game ON game.id = selected.game_id
        JOIN users white ON white.id = game.white_id
        JOIN users black ON black.id = game.black_id
        ORDER BY selected.last_message_id DESC
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<BigInt, _>(HUB_SECTION_LIMIT)
    .load(conn)
    .await
    .map_err(DbError::from)?;

    Ok(rows
        .into_iter()
        .map(|row| GameChannel {
            game_id: GameId(row.nanoid),
            label: format!("{} vs {}", row.white_name, row.black_name),
            finished: row.finished,
            last_message_id: row.last_message_id,
        })
        .collect())
}
