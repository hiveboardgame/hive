use crate::functions::accounts::account_response::AccountResponse;
use leptos::*;

#[server]
pub async fn get_account() -> Result<Option<AccountResponse>, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;

    let uuid = match uuid() {
        Ok(uuid) => uuid,
        Err(_) =>  return Ok(None),
    };
    let account_response = AccountResponse::from_uuid(&uuid, &pool()?).await?;
    Ok(Some(account_response))
}
