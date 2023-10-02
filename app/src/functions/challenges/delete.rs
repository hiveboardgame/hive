use leptos::*;
use uuid::Uuid;

#[server(DeleteChallenge)]
pub async fn delete_challenge(id: Uuid) -> Result<(), ServerFnError> {
    use crate::functions::db::pool;
    use db_lib::models::challenge::Challenge;
    let pool = pool()?;
    use crate::functions::auth::identity::identity;
    let user_id = identity()?.id()?;
    let challenge = Challenge::get(&id, &pool).await?;
    if challenge.challenger_uid != user_id {
        return Err(ServerFnError::ServerError(String::from(
            "Challenge can only be deleted by its creator",
        )));
    }
    challenge.delete(&pool).await?;
    Ok(())
}
