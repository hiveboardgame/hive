use leptos::prelude::*;

/// Log the calling user out.
///
/// - **Cookie path (SSR/hydrate)**: clears the actix-identity session cookie.
/// - **Bearer path (HiveGame mobile)**: the bearer is held client-side, so
///   we have nothing server-side to invalidate beyond the optional push
///   device row. The client clears its in-memory + localStorage token after
///   this call returns.
///
/// `device_id` is the push-device row id the mobile client persisted at
/// registration time. When present and resolvable, we delete that row here —
/// the bearer is still attached to this request, so `uuid()` resolves the
/// caller. Doing the cleanup in this server fn avoids the race where the
/// client's separate `unregister_device` call would fire *after* the bearer
/// is cleared and hit an unauthenticated endpoint.
///
/// All cleanup steps are best-effort: a failed DB delete or missing Identity
/// must not prevent the user from logging out client-side.
#[server(client = crate::client::ApiClient)]
pub async fn logout(device_id: Option<String>) -> Result<(), ServerFnError> {
    use crate::functions::{
        auth::identity::{identity, uuid},
        db::pool,
    };
    use ::uuid::Uuid;
    use db_lib::{get_conn, models::PushDevice};

    if let Ok(user_id) = uuid().await {
        if let Some(device_id) = device_id.filter(|s| !s.is_empty()) {
            if let Ok(device_uuid) = Uuid::parse_str(&device_id) {
                if let Ok(pool) = pool().await {
                    if let Ok(mut conn) = get_conn(&pool).await {
                        let _ = PushDevice::delete_for_user(device_uuid, user_id, &mut conn).await;
                    }
                }
            }
        }
    }

    if let Ok(id) = identity().await {
        id.logout();
    }

    Ok(())
}
