use db_lib::{
    db_error::DbError,
    helpers::{is_blocked, is_tournament_participant},
    models::{Game, Tournament, User},
    DbConn,
};
use shared_types::{
    canonical_dm_channel_id,
    direct_other_user_for_sender,
    is_valid_chat_channel_type,
    DirectChannelParseError,
    GameId,
    CHANNEL_TYPE_DIRECT,
    CHANNEL_TYPE_GAME_PLAYERS,
    CHANNEL_TYPE_GAME_SPECTATORS,
    CHANNEL_TYPE_GLOBAL,
    CHANNEL_TYPE_TOURNAMENT_LOBBY,
};
use uuid::Uuid;

#[derive(Debug)]
pub enum ChatSendAccessError {
    BadRequest(&'static str),
    Forbidden(&'static str),
    NotFound(&'static str),
    Internal {
        context: &'static str,
        error: DbError,
    },
}

fn map_not_found(
    err: DbError,
    not_found_message: &'static str,
    context: &'static str,
) -> ChatSendAccessError {
    match err {
        DbError::NotFound { .. } => ChatSendAccessError::NotFound(not_found_message),
        other => ChatSendAccessError::Internal {
            context,
            error: other,
        },
    }
}

fn parse_direct_other_user(channel_id: &str, sender_id: Uuid) -> Result<Uuid, ChatSendAccessError> {
    if channel_id.contains("::") {
        return match direct_other_user_for_sender(channel_id, sender_id) {
            Ok(other) => Ok(other),
            Err(DirectChannelParseError::InvalidFormat) => Err(ChatSendAccessError::BadRequest(
                "Invalid DM channel_id format",
            )),
            Err(DirectChannelParseError::InvalidUuid) => Err(ChatSendAccessError::BadRequest(
                "Invalid UUID in channel_id",
            )),
            Err(DirectChannelParseError::NotParticipant) => Err(ChatSendAccessError::Forbidden(
                "You are not a participant in this DM",
            )),
        };
    }

    match direct_other_user_for_sender(channel_id, sender_id) {
        Ok(other) => Ok(other),
        Err(DirectChannelParseError::InvalidUuid) => {
            Err(ChatSendAccessError::BadRequest("Invalid channel_id for DM"))
        }
        Err(DirectChannelParseError::InvalidFormat | DirectChannelParseError::NotParticipant) => {
            Err(ChatSendAccessError::BadRequest("Invalid channel_id for DM"))
        }
    }
}

async fn load_game_or_404(
    conn: &mut DbConn<'_>,
    channel_id: &str,
) -> Result<Game, ChatSendAccessError> {
    Game::find_by_game_id(&GameId(channel_id.to_string()), conn)
        .await
        .map_err(|e| map_not_found(e, "Game not found", "loading game"))
}

pub async fn authorize_chat_send_and_resolve_channel_id(
    conn: &mut DbConn<'_>,
    sender_id: Uuid,
    sender_is_admin: bool,
    channel_type: &str,
    channel_id: &str,
) -> Result<String, ChatSendAccessError> {
    if !is_valid_chat_channel_type(channel_type) {
        return Err(ChatSendAccessError::BadRequest("Invalid channel_type"));
    }

    if channel_type == CHANNEL_TYPE_GLOBAL && !sender_is_admin {
        return Err(ChatSendAccessError::Forbidden("Global chat requires admin"));
    }

    if channel_type == CHANNEL_TYPE_TOURNAMENT_LOBBY {
        Tournament::from_nanoid(channel_id, conn)
            .await
            .map_err(|e| map_not_found(e, "Tournament not found", "loading tournament"))?;

        let is_participant = is_tournament_participant(conn, sender_id, channel_id)
            .await
            .map_err(|e| ChatSendAccessError::Internal {
                context: "checking tournament membership",
                error: e,
            })?;
        if !is_participant {
            return Err(ChatSendAccessError::Forbidden(
                "Only tournament participants and organizers can send messages",
            ));
        }
    }

    if channel_type == CHANNEL_TYPE_GAME_PLAYERS {
        let game = load_game_or_404(conn, channel_id).await?;
        if sender_id != game.white_id && sender_id != game.black_id {
            return Err(ChatSendAccessError::Forbidden(
                "Only players can send to players chat",
            ));
        }
    }

    if channel_type == CHANNEL_TYPE_GAME_SPECTATORS {
        let game = load_game_or_404(conn, channel_id).await?;
        let is_player = sender_id == game.white_id || sender_id == game.black_id;
        if is_player && !game.finished {
            return Err(ChatSendAccessError::Forbidden(
                "Players cannot send to spectators chat while the game is ongoing",
            ));
        }
    }

    if channel_type != CHANNEL_TYPE_DIRECT {
        return Ok(channel_id.to_string());
    }

    let other_id = parse_direct_other_user(channel_id, sender_id)?;

    User::find_by_uuid(&other_id, conn)
        .await
        .map_err(|e| match e {
            DbError::NotFound { .. } => {
                ChatSendAccessError::Forbidden("You cannot send messages to this user")
            }
            other => ChatSendAccessError::Internal {
                context: "checking direct-message recipient",
                error: other,
            },
        })?;

    let recipient_blocked_sender =
        is_blocked(conn, other_id, sender_id)
            .await
            .map_err(|e| ChatSendAccessError::Internal {
                context: "checking direct-message block status",
                error: e,
            })?;
    if recipient_blocked_sender {
        return Err(ChatSendAccessError::Forbidden(
            "You cannot send messages to this user",
        ));
    }

    Ok(canonical_dm_channel_id(sender_id, other_id))
}
