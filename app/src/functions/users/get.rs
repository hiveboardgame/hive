use crate::functions::users::user_response::UserResponse;
use leptos::*;
use leptos::logging::log;

#[server(GetUser)]
pub async fn get_user(uuid: String) -> Result<UserResponse, ServerFnError> {
    log!("In User::Get");
    use crate::functions::db::pool;
    let pool = pool().expect("Failed to get pool");
    UserResponse::from_uid(&uuid, &pool).await
}
