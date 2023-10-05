use crate::functions::users::user_response::UserResponse;
use leptos::*;

#[server]
pub async fn get_user(uid: String) -> Result<UserResponse, ServerFnError> {
    use crate::functions::db::pool;
    let pool = pool()?;
    UserResponse::from_uid(&uid, &pool).await
}
