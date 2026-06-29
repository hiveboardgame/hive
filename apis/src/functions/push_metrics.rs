use leptos::prelude::*;
use shared_types::PushMetrics;

#[server]
pub async fn read_push_metrics() -> Result<PushMetrics, ServerFnError> {
    use crate::{
        functions::{auth::identity::ensure_admin, db::pool},
        notifications::PushTelemetry,
    };
    use actix_web::web::Data;
    use db_lib::get_conn;

    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    ensure_admin(&mut conn).await?;

    let req: actix_web::HttpRequest = leptos_actix::extract().await?;
    let telemetry = req
        .app_data::<Data<PushTelemetry>>()
        .ok_or_else(|| ServerFnError::new("push telemetry not registered"))?;
    Ok(telemetry.snapshot())
}
