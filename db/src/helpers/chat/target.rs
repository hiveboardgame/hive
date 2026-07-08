use crate::{
    db_error::DbError,
    models::{ChatChannelKind, Tournament, User, UserTournamentChatMute},
    schema::{chat_channels, games, tournaments_organizers, tournaments_users},
    DbConn,
};
use diesel::{dsl::exists, prelude::*, select};
use diesel_async::RunQueryDsl;
use shared_types::{
    ConversationKey,
    GameChatCapabilities,
    GameId,
    TournamentChatCapabilities,
    TournamentId,
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DbDirectChat {
    pub low_id: Uuid,
    pub high_id: Uuid,
}

impl DbDirectChat {
    fn new(user_id: Uuid, other_user_id: Uuid) -> Self {
        if user_id < other_user_id {
            Self {
                low_id: user_id,
                high_id: other_user_id,
            }
        } else {
            Self {
                low_id: other_user_id,
                high_id: user_id,
            }
        }
    }

    fn lookup_key(&self) -> String {
        format!("{}:{}", self.low_id, self.high_id)
    }
}

#[derive(Clone, Debug)]
pub struct DbGameChat {
    pub id: Uuid,
    pub game_id: GameId,
    pub white_id: Uuid,
    pub black_id: Uuid,
    pub finished: bool,
}

#[derive(Clone, Debug)]
pub struct DbTournamentChat {
    pub id: Uuid,
    pub tournament_id: TournamentId,
    pub name: String,
    pub access: TournamentChatCapabilities,
    pub muted: bool,
}

#[derive(Clone, Debug)]
pub struct DbChatTarget {
    pub key: ConversationKey,
    pub kind: ChatChannelKind,
    pub lookup_key: String,
    pub channel_id: Option<i64>,
    pub direct: Option<DbDirectChat>,
    pub game: Option<DbGameChat>,
    pub tournament: Option<DbTournamentChat>,
}

async fn lookup_channel_id(
    conn: &mut DbConn<'_>,
    kind: ChatChannelKind,
    lookup_key: &str,
) -> Result<Option<i64>, DbError> {
    chat_channels::table
        .filter(chat_channels::kind.eq(kind.as_str()))
        .filter(chat_channels::lookup_key.eq(lookup_key))
        .select(chat_channels::id)
        .first::<i64>(conn)
        .await
        .optional()
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

pub async fn get_tournament_chat_capabilities(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_id: Uuid,
) -> Result<TournamentChatCapabilities, DbError> {
    let is_site_admin = User::is_admin(&user_id, conn).await?;
    let is_organizer = select(exists(
        tournaments_organizers::table
            .filter(tournaments_organizers::organizer_id.eq(user_id))
            .filter(tournaments_organizers::tournament_id.eq(tournament_id)),
    ))
    .get_result(conn)
    .await
    .map_err(DbError::from)?;
    let is_participant = select(exists(
        tournaments_users::table
            .filter(tournaments_users::user_id.eq(user_id))
            .filter(tournaments_users::tournament_id.eq(tournament_id)),
    ))
    .get_result(conn)
    .await
    .map_err(DbError::from)?;

    Ok(TournamentChatCapabilities::new(
        is_site_admin,
        is_organizer,
        is_participant,
    ))
}

pub async fn get_tournament_thread_data(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<(String, bool, TournamentChatCapabilities), DbError> {
    let tournament = Tournament::from_nanoid(tournament_nanoid, conn).await?;
    let muted = UserTournamentChatMute::is_muted(conn, user_id, tournament.id).await?;
    let access = get_tournament_chat_capabilities(conn, user_id, tournament.id).await?;
    Ok((tournament.name, muted, access))
}

pub async fn resolve_chat_target(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    key: &ConversationKey,
) -> Result<DbChatTarget, DbError> {
    match key {
        ConversationKey::Direct(other_user_id) => {
            if *other_user_id == user_id {
                return Err(DbError::InvalidInput {
                    info: "Direct messages to yourself are not supported".to_string(),
                    error: "self direct message".to_string(),
                });
            }
            ensure_direct_peer_exists(conn, *other_user_id).await?;
            let kind = ChatChannelKind::Direct;
            let direct = DbDirectChat::new(user_id, *other_user_id);
            let lookup_key = direct.lookup_key();
            let channel_id = lookup_channel_id(conn, kind, &lookup_key).await?;
            Ok(DbChatTarget {
                key: key.clone(),
                kind,
                lookup_key,
                channel_id,
                direct: Some(direct),
                game: None,
                tournament: None,
            })
        }
        ConversationKey::Tournament(tournament_id) => {
            let tournament = Tournament::from_nanoid(&tournament_id.0, conn).await?;
            let access = get_tournament_chat_capabilities(conn, user_id, tournament.id).await?;
            let muted = UserTournamentChatMute::is_muted(conn, user_id, tournament.id).await?;
            let kind = ChatChannelKind::TournamentLobby;
            let lookup_key = tournament.id.to_string();
            let channel_id = lookup_channel_id(conn, kind, &lookup_key).await?;
            Ok(DbChatTarget {
                key: key.clone(),
                kind,
                lookup_key,
                channel_id,
                direct: None,
                game: None,
                tournament: Some(DbTournamentChat {
                    id: tournament.id,
                    tournament_id: tournament_id.clone(),
                    name: tournament.name,
                    access,
                    muted,
                }),
            })
        }
        ConversationKey::Game { game_id, thread } => {
            let (id, white_id, black_id, finished) = games::table
                .filter(games::nanoid.eq(&game_id.0))
                .select((games::id, games::white_id, games::black_id, games::finished))
                .first::<(Uuid, Uuid, Uuid, bool)>(conn)
                .await
                .map_err(DbError::from)?;
            let kind = ChatChannelKind::Game(*thread);
            let lookup_key = id.to_string();
            let channel_id = lookup_channel_id(conn, kind, &lookup_key).await?;
            Ok(DbChatTarget {
                key: key.clone(),
                kind,
                lookup_key,
                channel_id,
                direct: None,
                game: Some(DbGameChat {
                    id,
                    game_id: game_id.clone(),
                    white_id,
                    black_id,
                    finished,
                }),
                tournament: None,
            })
        }
        ConversationKey::Global => {
            let kind = ChatChannelKind::Global;
            let lookup_key = "global".to_string();
            let channel_id = lookup_channel_id(conn, kind, &lookup_key).await?;
            Ok(DbChatTarget {
                key: key.clone(),
                kind,
                lookup_key,
                channel_id,
                direct: None,
                game: None,
                tournament: None,
            })
        }
    }
}

pub fn can_user_read_target(user_id: Uuid, target: &DbChatTarget) -> bool {
    match &target.key {
        ConversationKey::Direct(other_user_id) => *other_user_id != user_id,
        ConversationKey::Tournament(_) => target
            .tournament
            .as_ref()
            .is_some_and(|tournament| tournament.access.can_read()),
        ConversationKey::Game { thread, .. } => target.game.as_ref().is_some_and(|game| {
            GameChatCapabilities::new(
                user_id == game.white_id || user_id == game.black_id,
                game.finished,
            )
            .can_read(*thread)
        }),
        ConversationKey::Global => true,
    }
}
