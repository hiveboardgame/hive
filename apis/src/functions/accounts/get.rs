use crate::responses::AccountResponse;
use leptos::prelude::*;
use server_fn::codec;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_account() -> Result<Option<AccountResponse>, ServerFnError> {
    use crate::functions::{auth::identity::optional_uuid, db::pool};
    use db_lib::get_conn;

    // Anonymous sessions are expected: return None without treating this as a server failure.
    let Some(uuid) = optional_uuid().await? else {
        return Ok(None);
    };

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let account_response = AccountResponse::from_uuid(&uuid, &mut conn).await?;
    Ok(Some(account_response))
}
