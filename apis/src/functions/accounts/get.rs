use crate::responses::AccountResponse;
use leptos::*;

#[server]
pub async fn get_account() -> Result<Option<AccountResponse>, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;

    let uuid = match uuid() {
        Ok(uuid) => uuid,
        Err(_) => return Ok(None),
    };
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    let account_response = AccountResponse::from_uuid(&uuid, &mut conn).await?;
    Ok(Some(account_response))
}
