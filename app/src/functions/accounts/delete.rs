use leptos::*;

#[server]
pub async fn delete_account() -> Result<(), ServerFnError> {
    use crate::functions::auth::{identity::uuid, logout::logout};
    use crate::functions::db::pool;
    use db_lib::models::user::User;
    let pool = pool()?;
    let uuid = uuid()?;
    let user = User::find_by_uuid(&uuid, &pool).await?;
    user.delete(&pool).await?;
    logout().await
}
