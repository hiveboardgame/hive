use leptos::prelude::*;

#[server]
pub async fn logout(device_endpoint: Option<String>) -> Result<(), ServerFnError> {
    use crate::functions::{
        auth::identity::{identity, uuid},
        db::pool,
    };
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

    if let Ok(id) = identity().await {
        id.logout();
    }

    Ok(())
}
