use leptos::prelude::*;

#[server]
pub async fn delete_account() -> Result<(), ServerFnError> {
    use crate::functions::auth::{identity::uuid, logout::logout};
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::User;
    let uuid = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let user = User::find_by_uuid(&uuid, &mut conn).await?;
    user.delete(&mut conn).await?;
    logout().await
}
