use crate::{
    db_error::DbError,
    models::{ChatChannelKind, User},
    schema::{chat_channels, games},
    DbConn,
};
use diesel::{
    prelude::*,
    sql_types::{Bool, Text, Uuid as SqlUuid},
};
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use shared_types::{ConversationKey, GameChatCapabilities, GameId, TournamentId};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DbGameChat {
    pub id: Uuid,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub finished: bool,
}

#[derive(Clone, Debug)]
pub enum DbChatTarget {
    Direct {
        other_user_id: Uuid,
        channel_id: Option<i64>,
        low_id: Uuid,
        high_id: Uuid,
    },
    Game {
        game_id: GameId,
        thread: shared_types::GameThread,
        channel_id: Option<i64>,
        game: DbGameChat,
    },
    Tournament {
        tournament_id: TournamentId,
        channel_id: Option<i64>,
        id: Uuid,
    },
    Global {
        channel_id: Option<i64>,
    },
}

impl DbChatTarget {
    pub fn channel_id(&self) -> Option<i64> {
        match self {
            Self::Direct { channel_id, .. }
            | Self::Game { channel_id, .. }
            | Self::Tournament { channel_id, .. }
            | Self::Global { channel_id } => *channel_id,
        }
    }

    pub fn kind(&self) -> ChatChannelKind {
        match self {
            Self::Direct { .. } => ChatChannelKind::Direct,
            Self::Game { thread, .. } => ChatChannelKind::Game(*thread),
            Self::Tournament { .. } => ChatChannelKind::TournamentLobby,
            Self::Global { .. } => ChatChannelKind::Global,
        }
    }

    fn assign_channel_id(&mut self, value: Option<i64>) {
        match self {
            Self::Direct { channel_id, .. }
            | Self::Game { channel_id, .. }
            | Self::Tournament { channel_id, .. }
            | Self::Global { channel_id } => *channel_id = value,
        }
    }
}

pub(super) async fn lookup_channel_id(
    conn: &mut AsyncPgConnection,
    target: &DbChatTarget,
) -> Result<Option<i64>, DbError> {
    let query = chat_channels::table
        .filter(chat_channels::kind.eq(target.kind().as_str()))
        .select(chat_channels::id);
    match target {
        DbChatTarget::Direct {
            low_id, high_id, ..
        } => query
            .filter(chat_channels::direct_user_low_id.eq(low_id))
            .filter(chat_channels::direct_user_high_id.eq(high_id))
            .first(conn)
            .await
            .optional(),
        DbChatTarget::Game { game, .. } => query
            .filter(chat_channels::game_id.eq(game.id))
            .first(conn)
            .await
            .optional(),
        DbChatTarget::Tournament { id, .. } => query
            .filter(chat_channels::tournament_id.eq(id))
            .first(conn)
            .await
            .optional(),
        DbChatTarget::Global { .. } => query.first(conn).await.optional(),
    }
    .map_err(DbError::from)
}

async fn ensure_direct_peer_exists(
    conn: &mut DbConn<'_>,
    other_user_id: Uuid,
) -> Result<(), DbError> {
    if User::uuid_exists(&other_user_id, conn).await? {
        Ok(())
    } else {
        Err(DbError::NotFound {
            reason: format!("Direct message peer {other_user_id} not found"),
        })
    }
}

pub async fn load_game_chat_capabilities(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    game_id: &GameId,
) -> Result<Option<GameChatCapabilities>, DbError> {
    match games::table
        .filter(games::nanoid.eq(&game_id.0))
        .select((games::white_id, games::black_id, games::finished))
        .first::<(Uuid, Uuid, bool)>(conn)
        .await
    {
        Ok((white_id, black_id, finished)) => Ok(Some(GameChatCapabilities::new(
            white_id == user_id || black_id == user_id,
            finished,
        ))),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(error) => Err(DbError::from(error)),
    }
}

#[derive(QueryableByName)]
pub(crate) struct TournamentChatAccess {
    #[diesel(sql_type = SqlUuid)]
    pub id: Uuid,
    #[diesel(sql_type = Text)]
    pub name: String,
    #[diesel(sql_type = Bool)]
    can_access: bool,
}

pub(crate) async fn authorize_tournament_chat_access(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<TournamentChatAccess, DbError> {
    let access = diesel::sql_query(
        r#"
        SELECT
            t.id,
            t.name,
            (
                EXISTS (
                    SELECT 1 FROM users u
                    WHERE u.id = $1 AND u.admin = TRUE
                )
                OR EXISTS (
                    SELECT 1 FROM tournaments_organizers o
                    WHERE o.tournament_id = t.id AND o.organizer_id = $1
                )
                OR EXISTS (
                    SELECT 1 FROM tournaments_users tu
                    WHERE tu.tournament_id = t.id AND tu.user_id = $1
                )
            ) AS can_access
        FROM tournaments t
        WHERE t.nanoid = $2
        "#,
    )
    .bind::<SqlUuid, _>(user_id)
    .bind::<Text, _>(tournament_nanoid)
    .get_result::<TournamentChatAccess>(conn)
    .await
    .optional()
    .map_err(DbError::from)?
    .ok_or_else(|| DbError::NotFound {
        reason: format!("Tournament {tournament_nanoid} not found"),
    })?;
    if access.can_access {
        Ok(access)
    } else {
        Err(DbError::Unauthorized)
    }
}

pub async fn get_tournament_thread_data(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<String, DbError> {
    authorize_tournament_chat_access(conn, user_id, tournament_nanoid)
        .await
        .map(|access| access.name)
}

pub async fn resolve_chat_target(
    conn: &mut DbConn<'_>,
    user_id: Option<Uuid>,
    key: &ConversationKey,
) -> Result<DbChatTarget, DbError> {
    let mut target = match key {
        ConversationKey::Direct(other_user_id) => {
            let Some(user_id) = user_id else {
                return Err(DbError::Unauthorized);
            };
            if *other_user_id == user_id {
                return Err(DbError::InvalidInput {
                    info: "Direct messages to yourself are not supported".to_string(),
                    error: "self direct message".to_string(),
                });
            }
            ensure_direct_peer_exists(conn, *other_user_id).await?;
            let (low_id, high_id) = if user_id < *other_user_id {
                (user_id, *other_user_id)
            } else {
                (*other_user_id, user_id)
            };
            DbChatTarget::Direct {
                other_user_id: *other_user_id,
                channel_id: None,
                low_id,
                high_id,
            }
        }
        ConversationKey::Tournament(tournament_id) => {
            let Some(user_id) = user_id else {
                return Err(DbError::Unauthorized);
            };
            let access = authorize_tournament_chat_access(conn, user_id, &tournament_id.0).await?;
            DbChatTarget::Tournament {
                tournament_id: tournament_id.clone(),
                channel_id: None,
                id: access.id,
            }
        }
        ConversationKey::Game { game_id, thread } => {
            let (id, white_id, black_id, finished) = games::table
                .filter(games::nanoid.eq(&game_id.0))
                .select((games::id, games::white_id, games::black_id, games::finished))
                .first::<(Uuid, Uuid, Uuid, bool)>(conn)
                .await
                .map_err(DbError::from)?;
            let target = DbChatTarget::Game {
                game_id: game_id.clone(),
                thread: *thread,
                channel_id: None,
                game: DbGameChat {
                    id,
                    white_id,
                    black_id,
                    finished,
                },
            };
            let capabilities = GameChatCapabilities::new(
                user_id.is_some_and(|user_id| user_id == white_id || user_id == black_id),
                finished,
            );
            if !capabilities.can_read(*thread) {
                return Err(DbError::Unauthorized);
            }
            target
        }
        ConversationKey::Global => DbChatTarget::Global { channel_id: None },
    };
    let channel_id = lookup_channel_id(conn, &target).await?;
    target.assign_channel_id(channel_id);
    Ok(target)
}
