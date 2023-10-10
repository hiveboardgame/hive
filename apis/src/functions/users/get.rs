use crate::functions::users::user_response::UserResponse;
use leptos::*;
use uuid::Uuid;

#[server]
pub async fn get_user(uuid: Uuid) -> Result<UserResponse, ServerFnError> {
    use crate::functions::db::pool;
    let pool = pool()?;
    UserResponse::from_uuid(&uuid, &pool).await
}
