use actix_web::web::Data;
use db_lib::DbPool;
use leptos::*;

pub fn pool() -> Result<DbPool, ServerFnError> {
    let req = use_context::<actix_web::HttpRequest>()
        .ok_or("Failed to get HttpRequest")
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?;
    let pool = req
        .app_data::<Data<DbPool>>()
        .ok_or("Failed to get pool")
        .map_err(|e| ServerFnError::ServerError(e.to_string()))?
        .get_ref()
        .clone();
    Ok(pool)
}
