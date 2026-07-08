#[cfg(feature = "ssr")]
use db_lib::db_error::DbError;
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use log::error;
use server_fn::codec;
#[cfg(feature = "ssr")]
use shared_types::TournamentId;
use uuid::Uuid;

#[cfg(feature = "ssr")]
fn blocks_mutes_unexpected_error(
    context: &'static str,
    err: impl std::fmt::Display,
) -> ServerFnError {
    error!("blocks/mutes server function failed while {context}: {err}");
    ServerFnError::new("Unable to update block or mute settings")
}

#[cfg(feature = "ssr")]
fn blocks_mutes_user_action_error(context: &'static str, err: DbError) -> ServerFnError {
    if !matches!(
        err,
        DbError::InvalidInput { .. } | DbError::NotFound { .. } | DbError::Unauthorized
    ) {
        error!("blocks/mutes server function failed while {context}: {err}");
    }
    ServerFnError::new("Unable to update block or mute settings")
}

#[cfg(feature = "ssr")]
async fn dispatch_user_settings_update(user_id: Uuid, update: crate::common::UserSettingsUpdate) {
    use crate::{
        common::{ServerMessage, ServerResult},
        websocket::{MessageDestination, WsHub},
    };
    use actix_web::web::Data;
    use bytes::Bytes;
    use codee::{binary::MsgpackSerdeCodec, Encoder};

    let Ok(hub) = leptos_actix::extract::<Data<std::sync::Arc<WsHub>>>().await else {
        return;
    };
    let message = ServerResult::Ok(Box::new(ServerMessage::UserSettings(update)));
    let Ok(serialized) = MsgpackSerdeCodec::encode(&message) else {
        return;
    };
    hub.dispatch(
        &MessageDestination::User(user_id),
        Bytes::from(serialized),
        None,
    )
    .await;
}

#[cfg(feature = "ssr")]
async fn auth_pool() -> Result<(Uuid, db_lib::DbPool), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};

    Ok((uuid().await?, pool().await?))
}

#[cfg(feature = "ssr")]
async fn blocks_mutes_conn(pool: &db_lib::DbPool) -> Result<db_lib::DbConn<'_>, ServerFnError> {
    db_lib::get_conn(pool)
        .await
        .map_err(|err| blocks_mutes_unexpected_error("getting database connection", err))
}

#[cfg(feature = "ssr")]
async fn set_block_setting(blocked_id: Uuid, blocked: bool) -> Result<(), ServerFnError> {
    use crate::common::UserSettingsUpdate;
    use db_lib::models::UserBlock;

    let (blocker_id, pool) = auth_pool().await?;
    let mut conn = blocks_mutes_conn(&pool).await?;
    if blocked {
        UserBlock::block(&mut conn, blocker_id, blocked_id)
            .await
            .map_err(|err| blocks_mutes_user_action_error("blocking user", err))?;
    } else {
        UserBlock::unblock(&mut conn, blocker_id, blocked_id)
            .await
            .map_err(|err| blocks_mutes_user_action_error("unblocking user", err))?;
    }
    dispatch_user_settings_update(
        blocker_id,
        UserSettingsUpdate::BlockedUser {
            user_id: blocked_id,
            blocked,
        },
    )
    .await;
    Ok(())
}

#[cfg(feature = "ssr")]
async fn set_tournament_mute_setting(
    tournament_id: String,
    muted: bool,
) -> Result<(), ServerFnError> {
    use crate::common::UserSettingsUpdate;
    use db_lib::helpers::{mute_tournament_chat, unmute_tournament_chat};

    let (user_id, pool) = auth_pool().await?;
    let tournament_id = tournament_id.trim().to_string();
    let mut conn = blocks_mutes_conn(&pool).await?;
    if muted {
        mute_tournament_chat(&mut conn, user_id, &tournament_id)
            .await
            .map_err(|err| blocks_mutes_user_action_error("muting tournament chat", err))?;
    } else {
        unmute_tournament_chat(&mut conn, user_id, &tournament_id)
            .await
            .map_err(|err| blocks_mutes_user_action_error("unmuting tournament chat", err))?;
    }
    dispatch_user_settings_update(
        user_id,
        UserSettingsUpdate::TournamentChatMuted {
            tournament_id: TournamentId(tournament_id),
            muted,
        },
    )
    .await;
    Ok(())
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn add_block(blocked_id: Uuid) -> Result<(), ServerFnError> {
    set_block_setting(blocked_id, true).await
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn remove_block(blocked_id: Uuid) -> Result<(), ServerFnError> {
    set_block_setting(blocked_id, false).await
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_blocked_user_ids() -> Result<Vec<Uuid>, ServerFnError> {
    use db_lib::models::UserBlock;

    let (blocker_id, pool) = auth_pool().await?;
    let mut conn = blocks_mutes_conn(&pool).await?;
    UserBlock::blocked_user_ids(&mut conn, blocker_id)
        .await
        .map_err(|err| blocks_mutes_user_action_error("loading blocked users", err))
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn mute_tournament_chat(tournament_id: String) -> Result<(), ServerFnError> {
    set_tournament_mute_setting(tournament_id, true).await
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn unmute_tournament_chat(tournament_id: String) -> Result<(), ServerFnError> {
    set_tournament_mute_setting(tournament_id, false).await
}
