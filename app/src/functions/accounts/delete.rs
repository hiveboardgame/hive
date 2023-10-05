use leptos::*;

#[server]
pub async fn delete_account() -> Result<(), ServerFnError> {
    use crate::functions::auth::{identity::identity, logout::logout};
    use crate::functions::db::pool;
    use db_lib::models::user::User;
    let pool = pool()?;
    match identity() {
        Ok(identity) => {
            let uid = identity.id().unwrap();
            let user = User::find_by_uid(&uid, &pool).await?;
            user.delete(&pool).await?;
            logout().await
        }
        Err(e) => Err(e),
    }
}
