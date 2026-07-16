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
    hub.dispatch(&MessageDestination::User(user_id), Bytes::from(serialized))
        .await;
}

#[cfg(feature = "ssr")]
async fn set_block_setting(blocked_id: Uuid, blocked: bool) -> Result<bool, ServerFnError> {
    use crate::{
        common::UserSettingsUpdate,
        functions::{auth::identity::uuid, db::pool},
    };
    use db_lib::helpers::{block_user, unblock_user};

    let blocker_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = db_lib::get_conn(&pool)
        .await
        .map_err(|err| blocks_mutes_unexpected_error("getting database connection", err))?;
    if blocked {
        block_user(&mut conn, blocker_id, blocked_id)
            .await
            .map_err(|err| blocks_mutes_user_action_error("blocking user", err))?;
    } else {
        unblock_user(&mut conn, blocker_id, blocked_id)
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
    Ok(blocked)
}

#[cfg(feature = "ssr")]
async fn set_tournament_mute_setting(
    tournament_id: String,
    muted: bool,
) -> Result<bool, ServerFnError> {
    use crate::{
        common::UserSettingsUpdate,
        functions::{auth::identity::uuid, db::pool},
    };
    use db_lib::helpers::set_tournament_chat_muted;

    let user_id = uuid().await?;
    let pool = pool().await?;
    let tournament_id = tournament_id.trim().to_string();
    let mut conn = db_lib::get_conn(&pool)
        .await
        .map_err(|err| blocks_mutes_unexpected_error("getting database connection", err))?;
    set_tournament_chat_muted(&mut conn, user_id, &tournament_id, muted)
        .await
        .map_err(|err| blocks_mutes_user_action_error("updating tournament chat mute", err))?;
    dispatch_user_settings_update(
        user_id,
        UserSettingsUpdate::TournamentChatMuted {
            tournament_id: TournamentId(tournament_id),
            muted,
        },
    )
    .await;
    Ok(muted)
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn set_user_blocked(blocked_id: Uuid, blocked: bool) -> Result<bool, ServerFnError> {
    set_block_setting(blocked_id, blocked).await
}

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn set_tournament_chat_muted(
    tournament_id: String,
    muted: bool,
) -> Result<bool, ServerFnError> {
    set_tournament_mute_setting(tournament_id, muted).await
}
