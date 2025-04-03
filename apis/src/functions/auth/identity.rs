use actix_identity::Identity;
use leptos::prelude::*;
use uuid::Uuid;

pub async fn identity() -> Result<Identity, ServerFnError> {
    leptos_actix::extract().await?
}

pub async fn uuid() -> Result<Uuid, ServerFnError> {
    let id_str = identity().await?.id()?;
    Uuid::parse_str(&id_str)
        .map_err(|e| ServerFnError::new(format!("Could not retrieve Uuid from identity: {e}")))
}

#[cfg(feature = "ssr")]
pub async fn ensure_admin(conn: &mut db_lib::DbConn<'_>) -> Result<(), ServerFnError> {
    use db_lib::models::User;
    let user = User::find_by_uuid(&uuid().await?, conn).await?;
    if !user.admin {
        Err(ServerFnError::new("You are not an admin"))
    } else {
        Ok(())
    }
}
