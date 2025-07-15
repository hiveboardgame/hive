use leptos::prelude::*;

#[server]
pub async fn mark_schedule_seen(schedule_id: String) -> Result<(), ServerFnError> {
    use crate::functions::auth::identity::uuid;
    use crate::functions::db::pool;
    use db_lib::get_conn;
    use db_lib::models::Schedule;
    use uuid::Uuid;
    let user_id = uuid().await?;
    let pool = pool().await?;
    let mut conn = get_conn(&pool).await?;
    let schedule_uuid = Uuid::parse_str(&schedule_id).map_err(ServerFnError::new)?;
    Schedule::mark_notified(schedule_uuid, user_id, &mut conn)
        .await
        .map_err(ServerFnError::new)?;

    Ok(())
}
