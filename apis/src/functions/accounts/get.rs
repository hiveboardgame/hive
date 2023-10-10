use crate::functions::accounts::account_response::AccountResponse;
use leptos::*;

#[server]
pub async fn get_account() -> Result<AccountResponse, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;

    let uuid = uuid()?;
    AccountResponse::from_uuid(&uuid, &pool()?).await
}
