use db_lib::{
    db_error::DbError,
    helpers::{is_blocked, is_tournament_participant},
    models::{Game, Tournament, User},
    DbConn,
};
use shared_types::{
    ChannelKey,
    ChannelType,
    GameId,
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
    let Some((a_raw, b_raw)) = channel_id.split_once("::") else {
        return Err(ChatSendAccessError::BadRequest("Invalid DM channel_id format"));
    };
    if b_raw.contains("::") {
        return Err(ChatSendAccessError::BadRequest("Invalid DM channel_id format"));
    }

    let a = Uuid::parse_str(a_raw)
        .map_err(|_| ChatSendAccessError::BadRequest("Invalid UUID in channel_id"))?;
    let b = Uuid::parse_str(b_raw)
        .map_err(|_| ChatSendAccessError::BadRequest("Invalid UUID in channel_id"))?;

    if a == b {
        return Err(ChatSendAccessError::BadRequest(
            "Direct messages to yourself are not supported",
        ));
    }

    let other_id = match (a == sender_id, b == sender_id) {
        (true, false) => b,
        (false, true) => a,
        _ => {
            return Err(ChatSendAccessError::Forbidden(
                "You are not a participant in this DM",
            ));
        }
    };

    Ok(other_id)
}

async fn load_game_or_404(
    conn: &mut DbConn<'_>,
    channel_id: &str,
) -> Result<Game, ChatSendAccessError> {
    Game::find_by_game_id(&GameId(channel_id.to_string()), conn)
        .await
        .map_err(|e| map_not_found(e, "Game not found", "loading game"))
}

pub async fn authorize_chat_send_and_resolve_channel_key(
    conn: &mut DbConn<'_>,
    sender_id: Uuid,
    sender_is_admin: bool,
    channel_type: &str,
    channel_id: &str,
) -> Result<ChannelKey, ChatSendAccessError> {
    let channel_type = channel_type
        .parse::<ChannelType>()
        .map_err(|_| ChatSendAccessError::BadRequest("Invalid channel_type"))?;
    if channel_type == ChannelType::Direct {
        let other_id = parse_direct_other_user(channel_id, sender_id)?;
        let resolved_channel_key =
            ChannelKey::normalized(channel_type, channel_id).ok_or(
                ChatSendAccessError::BadRequest("Invalid DM channel_id format"),
            )?;

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

        return Ok(resolved_channel_key);
    }

    let resolved_channel_key = ChannelKey::normalized(channel_type, channel_id)
        .ok_or(ChatSendAccessError::BadRequest("Invalid channel_id"))?;
    let resolved_channel_id = resolved_channel_key.channel_id.as_str();

    if channel_type == ChannelType::Global && !sender_is_admin {
        return Err(ChatSendAccessError::Forbidden("Global chat requires admin"));
    }

    if channel_type == ChannelType::TournamentLobby {
        Tournament::from_nanoid(resolved_channel_id, conn)
            .await
            .map_err(|e| map_not_found(e, "Tournament not found", "loading tournament"))?;

        let can_chat = sender_is_admin
            || is_tournament_participant(conn, sender_id, resolved_channel_id)
                .await
                .map_err(|e| ChatSendAccessError::Internal {
                    context: "checking tournament membership",
                    error: e,
                })?;
        if !can_chat {
            return Err(ChatSendAccessError::Forbidden(
                "Only tournament participants and organizers can send messages",
            ));
        }
    }

    if channel_type == ChannelType::GamePlayers {
        let game = load_game_or_404(conn, resolved_channel_id).await?;
        if sender_id != game.white_id && sender_id != game.black_id {
            return Err(ChatSendAccessError::Forbidden(
                "Only players can send to players chat",
            ));
        }
    }

    if channel_type == ChannelType::GameSpectators {
        let game = load_game_or_404(conn, resolved_channel_id).await?;
        let is_player = sender_id == game.white_id || sender_id == game.black_id;
        if is_player && !game.finished {
            return Err(ChatSendAccessError::Forbidden(
                "Players cannot send to spectators chat while the game is ongoing",
            ));
        }
    }

    Ok(resolved_channel_key)
}
