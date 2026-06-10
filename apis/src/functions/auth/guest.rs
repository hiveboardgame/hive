use crate::responses::AccountResponse;
use leptos::prelude::*;

/// Provisions an ephemeral guest account and logs it in via the session cookie.
/// Idempotent: if the caller already has a session (guest or full), their
/// existing account is returned instead of stacking a new guest row.
#[server(input = server_fn::codec::Cbor, output = server_fn::codec::Cbor)]
pub async fn guest_login() -> Result<AccountResponse, ServerFnError> {
    use crate::functions::{auth::identity::uuid, db::pool};
    use actix_identity::Identity;
    use actix_web::HttpMessage;
    use db_lib::{
        get_conn,
        models::{NewUser, User},
    };

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;

    // Already have a session? Return that account, don't mint another guest.
    if let Ok(existing) = uuid().await {
        if let Ok(account) = AccountResponse::from_uuid(&existing, &mut conn).await {
            return Ok(account);
        }
    }

    // The synthetic username/email are random; on the astronomically unlikely
    // unique-constraint collision, just try again.
    let mut last_err = None;
    let mut user = None;
    for _ in 0..5 {
        match User::create(NewUser::new_guest(), &mut conn).await {
            Ok(u) => {
                user = Some(u);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let user = match user {
        Some(u) => u,
        None => {
            return Err(ServerFnError::new(
                last_err
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Could not create guest".to_string()),
            ))
        }
    };

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    Identity::login(&req.extensions(), user.id.to_string())?;
    AccountResponse::from_uuid(&user.id, &mut conn).await
}
