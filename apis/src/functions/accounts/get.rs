use crate::responses::AccountResponse;
use leptos::prelude::*;
use server_fn::codec;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_account() -> Result<Option<AccountResponse>, ServerFnError> {
    use crate::functions::{auth::identity::identity, db::pool};
    use db_lib::get_conn;
    use uuid::Uuid;

    let identity = match identity().await {
        Ok(identity) => identity,
        Err(_) => return Ok(None),
    };
    let id = match identity.id() {
        Ok(id) => id,
        Err(_) => return Ok(None),
    };
    let uuid = Uuid::parse_str(&id)
        .map_err(|e| ServerFnError::new(format!("Could not retrieve Uuid from identity: {e}")))?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let account_response = AccountResponse::from_uuid(&uuid, &mut conn).await?;
    Ok(Some(account_response))
}
