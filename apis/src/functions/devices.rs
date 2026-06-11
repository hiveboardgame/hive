use leptos::prelude::*;
use uuid::Uuid;

/// Register the push token the OS handed us. Keyed on (platform, device_token)
/// at the DB layer, so re-registering the same token rebinds it to the
/// calling user instead of duplicating rows — covers token rotation and
/// "different account on the same phone" without extra client logic.
///
/// Returns the device row id. The mobile client persists this so it can call
/// `unregister_device` on logout / opt-out without having to look up by token.
#[server(client = crate::client::ApiClient)]
pub async fn register_device(
    platform: String,
    device_token: String,
    app_version: String,
    locale: String,
) -> Result<Uuid, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{
        get_conn,
        models::{NewPushDevice, PushDevice},
    };

    // Fail early with a clear message — the DB CHECK would catch this but
    // would surface as an opaque diesel error.
    if platform != "apns" && platform != "fcm" {
        return Err(ServerFnError::new("platform must be 'apns' or 'fcm'"));
    }

    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    let device = PushDevice::upsert(
        NewPushDevice {
            user_id,
            platform,
            device_token,
            app_version,
            locale,
        },
        &mut conn,
    )
    .await
    .map_err(ServerFnError::new)?;

    Ok(device.id)
}

/// Drop a device row for the calling user. The user_id filter in
/// `PushDevice::delete_for_user` means a user can't unregister someone else's
/// device even if they guess the UUID.
#[server(client = crate::client::ApiClient)]
pub async fn unregister_device(device_id: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::PushDevice};

    let device_uuid = Uuid::parse_str(&device_id).map_err(ServerFnError::new)?;
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    PushDevice::delete_for_user(device_uuid, user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}
