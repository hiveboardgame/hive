use super::chat::get_tournament_chat_capabilities;
use crate::{
    db_error::DbError,
    models::{Tournament, UserTournamentChatMute},
    DbConn,
};
use uuid::Uuid;

pub async fn mute_tournament_chat(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<(), DbError> {
    let tournament = Tournament::from_nanoid(tournament_nanoid, conn).await?;
    let access = get_tournament_chat_capabilities(conn, user_id, tournament.id).await?;
    if !access.can_read() {
        return Err(DbError::Unauthorized);
    }
    UserTournamentChatMute::mute(conn, user_id, tournament.id).await
}

pub async fn unmute_tournament_chat(
    conn: &mut DbConn<'_>,
    user_id: Uuid,
    tournament_nanoid: &str,
) -> Result<(), DbError> {
    let tournament = Tournament::from_nanoid(tournament_nanoid, conn).await?;
    let access = get_tournament_chat_capabilities(conn, user_id, tournament.id).await?;
    if !access.can_read() {
        return Err(DbError::Unauthorized);
    }
    UserTournamentChatMute::unmute(conn, user_id, tournament.id).await
}
