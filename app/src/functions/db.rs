use actix_web::web::Data;
use db_lib::DbPool;
use leptos::*;

pub fn pool() -> Result<DbPool, ()> {
    let req = use_context::<actix_web::HttpRequest>().expect("Failed to get req");
    let pool = req.app_data::<Data<DbPool>>().expect("Failed to get Pool").get_ref().clone();
    Ok(pool)
}
