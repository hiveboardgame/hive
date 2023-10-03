use crate::functions::accounts::account_response::AccountResponse;
use leptos::*;

#[server(GetAccount)]
pub async fn get_account() -> Result<AccountResponse, ServerFnError> {
    use crate::functions::auth::identity::identity;
    use crate::functions::db::pool;

    match identity() {
        Ok(identity) => {
            let uuid = identity.id().unwrap();
            AccountResponse::from_uid(&uuid, &pool()?).await
        }
        Err(e) => Err(e),
    }
}
