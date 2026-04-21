use crate::responses::AccountResponse;
use leptos::prelude::*;
use server_fn::codec;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_account() -> Result<Option<AccountResponse>, ServerFnError> {
    use crate::functions::db::pool;
    use actix_identity::Identity;
    use db_lib::get_conn;
    use uuid::Uuid;

    // Anonymous sessions are expected: return None without treating this as a server failure.
    let identity: Option<Identity> = leptos_actix::extract().await?;
    let Some(identity) = identity else {
        return Ok(None);
    };
    let identity = identity.id()?;
    let uuid = Uuid::parse_str(&identity)
        .map_err(|e| ServerFnError::new(format!("Could not retrieve Uuid from identity: {e}")))?;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let account_response = AccountResponse::from_uuid(&uuid, &mut conn).await?;
    Ok(Some(account_response))
}
