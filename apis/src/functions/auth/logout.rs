#[cfg(feature = "ssr")]
use crate::security::csrf::reset_session;
use crate::security::csrf::CsrfClient;
#[cfg(feature = "ssr")]
use actix_session::Session;
use leptos::prelude::*;

#[server(client = CsrfClient)]
pub async fn logout(device_endpoint: Option<String>) -> Result<(), ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use db_lib::{get_conn, models::PushDevice};

    if let Ok(user_id) = uuid().await {
        if let Some(endpoint) = device_endpoint.filter(|s| !s.is_empty()) {
            if let Ok(pool) = pool().await {
                if let Ok(mut conn) = get_conn(&pool).await {
                    let _ =
                        PushDevice::revoke_by_token_for_user(user_id, &endpoint, &mut conn).await;
                }
            }
        }
    }

    let session: Session = leptos_actix::extract().await?;
    reset_session(&session)?;

    Ok(())
}
