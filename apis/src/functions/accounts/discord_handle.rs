use crate::responses::AccountResponse;
use leptos::*;

#[server]
pub async fn discord_handle(
    discord_handle: String,
) -> Result<AccountResponse, ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::User;

    let uuid = uuid()?;
    let pool = pool()?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_uuid(&uuid, &mut conn).await?;

    user.set_discord_handle(discord_handle, &mut conn).await?;
    AccountResponse::from_uuid(&user.id, &mut conn).await
}
