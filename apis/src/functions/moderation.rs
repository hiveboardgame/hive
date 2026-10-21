use crate::security::csrf::CsrfClient;
use leptos::prelude::*;
use server_fn::codec;
use uuid::Uuid;

#[server(client = CsrfClient, input = codec::Cbor, output = codec::Cbor)]
pub async fn set_chat_muted(user_id: Uuid, muted: bool) -> Result<bool, ServerFnError> {
    use crate::{
        common::{UiError, UserSettingsUpdate},
        functions::{
            auth::identity::ensure_admin,
            blocks_mutes::dispatch_user_settings_update,
            db::pool,
        },
    };
    use db_lib::{get_conn, models::User};
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;
    User::set_chat_muted(user_id, muted, &mut conn)
        .await
        .map_err(|error| {
            log::error!("setting admin chat mute: {error}");
            ServerFnError::new(UiError::BlockMuteFailed.encode())
        })?;
    dispatch_user_settings_update(user_id, UserSettingsUpdate::ChatMuted).await;
    Ok(muted)
}
