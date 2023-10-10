use leptos::*;
use uuid::Uuid;

#[server]
pub async fn delete_challenge(id: Uuid) -> Result<(), ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    let pool = pool()?;
    use crate::functions::auth::identity::uuid;
    let user_id = uuid()?;
    let challenge = Challenge::find_by_uuid(&id, &pool).await?;
    if challenge.challenger_id != user_id {
        return Err(ServerFnError::ServerError(String::from(
            "Challenge can only be deleted by its creator",
        )));
    }
    challenge.delete(&pool).await?;
    Ok(())
}
