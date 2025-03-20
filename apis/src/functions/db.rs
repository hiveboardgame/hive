use actix_web::web::Data;
use db_lib::DbPool;
use leptos::prelude::*;

pub async fn pool() -> Result<DbPool, ServerFnError> {
    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    let pool = req
        .app_data::<Data<DbPool>>()
        .ok_or("Failed to get pool")
        .map_err(ServerFnError::new)?
        .get_ref()
        .clone();
    Ok(pool)
}
