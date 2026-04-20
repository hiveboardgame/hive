//! Server functions for block list and tournament chat mutes. All use session user (only self can act).

use leptos::prelude::*;
#[cfg(feature = "ssr")]
use log::error;
use server_fn::codec;
use uuid::Uuid;

#[cfg(feature = "ssr")]
fn generic_blocks_mutes_error(context: &'static str, err: impl std::fmt::Display) -> ServerFnError {
    error!("blocks/mutes server fn failed while {context}: {err}");
    ServerFnError::new("Unable to update block or mute settings")
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn add_block(blocked_id: Uuid) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, helpers::block_user};
    let blocker_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_blocks_mutes_error("getting a database connection", err))?;
    block_user(&mut conn, blocker_id, blocked_id)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn remove_block(blocked_id: Uuid) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, helpers::unblock_user};
    let blocker_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_blocks_mutes_error("getting a database connection", err))?;
    unblock_user(&mut conn, blocker_id, blocked_id)
        .await
        .map_err(ServerFnError::new)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_blocked_user_ids() -> Result<Vec<Uuid>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, helpers::get_blocked_user_ids as db_get_blocked};
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_blocks_mutes_error("getting a database connection", err))?;
    db_get_blocked(&mut conn, user_id)
        .await
        .map_err(|err| generic_blocks_mutes_error("loading blocked users", err))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn mute_tournament_chat(tournament_id: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, helpers::mute_tournament_chat as db_mute};
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_blocks_mutes_error("getting a database connection", err))?;
    db_mute(&mut conn, user_id, tournament_id.trim())
        .await
        .map_err(|err| generic_blocks_mutes_error("muting tournament chat", err))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn unmute_tournament_chat(tournament_id: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, helpers::unmute_tournament_chat as db_unmute};
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool)
        .await
        .map_err(|err| generic_blocks_mutes_error("getting a database connection", err))?;
    db_unmute(&mut conn, user_id, tournament_id.trim())
        .await
        .map_err(|err| generic_blocks_mutes_error("unmuting tournament chat", err))
}
