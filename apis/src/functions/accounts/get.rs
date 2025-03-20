use crate::responses::AccountResponse;
use leptos::prelude::*;
use server_fn::codec;

#[server(input = codec::Cbor, output = codec::Cbor)]
pub async fn get_account() -> Result<AccountResponse, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;

    let uuid = match uuid().await {
        Ok(uuid) => uuid,
        Err(e) => return Err(e),
    };
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let account_response = AccountResponse::from_uuid(&uuid, &mut conn).await?;
    Ok(account_response)
}
