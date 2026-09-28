#[cfg(feature = "ssr")]
use crate::notifications::web_push;
use crate::{responses::PushDeviceResponse, security::csrf::CsrfClient};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistration {
    pub platform: String,
    pub device_token: String,
    pub app_version: String,
    pub locale: String,
    pub p256dh: Option<String>,
    pub auth: Option<String>,
    pub explicit: bool,
}

#[cfg(feature = "ssr")]
pub fn new_web_push_device(
    user_id: Uuid,
    endpoint: String,
    p256dh: String,
    auth: String,
    app_version: String,
    locale: String,
) -> Result<db_lib::models::NewPushDevice, String> {
    web_push::validate_push_endpoint(&endpoint)?;
    Ok(db_lib::models::NewPushDevice {
        user_id,
        platform: "web".to_string(),
        device_token: endpoint,
        app_version,
        locale,
        p256dh: Some(p256dh),
        auth: Some(auth),
    })
}

#[server(client = CsrfClient)]
pub async fn register_device(registration: DeviceRegistration) -> Result<Uuid, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::PushDevice};

    let DeviceRegistration {
        platform,
        device_token,
        app_version,
        locale,
        p256dh,
        auth,
        explicit,
    } = registration;

    if platform != "web" {
        return Err(ServerFnError::new("platform must be 'web'"));
    }
    let (Some(p256dh), Some(auth)) = (p256dh, auth) else {
        return Err(ServerFnError::new(
            "web push registration requires p256dh and auth keys",
        ));
    };

    let user_id = uuid().await?;
    let new_device = new_web_push_device(user_id, device_token, p256dh, auth, app_version, locale)
        .map_err(ServerFnError::new)?;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let device = PushDevice::upsert(new_device, explicit, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(device.id)
}

#[server(client = CsrfClient)]
pub async fn list_devices(
    current_endpoint: Option<String>,
) -> Result<Vec<PushDeviceResponse>, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::PushDevice};

    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let devices = PushDevice::find_for_user(user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;
    Ok(devices
        .into_iter()
        .map(|d| PushDeviceResponse::from_model(d, current_endpoint.as_deref()))
        .collect())
}

#[server(client = CsrfClient)]
pub async fn unregister_device(device_id: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::PushDevice};

    let device_uuid = Uuid::parse_str(&device_id).map_err(ServerFnError::new)?;
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    PushDevice::revoke_for_user(device_uuid, user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}

#[server(client = CsrfClient)]
pub async fn unregister_current_device(endpoint: String) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::PushDevice};

    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    PushDevice::delete_by_token_for_user(user_id, &endpoint, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}

#[server(client = CsrfClient)]
pub async fn get_vapid_public_key() -> Result<Option<String>, ServerFnError> {
    Ok(web_push::cached_public_key().map(str::to_string))
}
