use crate::responses::UserResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AccountResponse {
    pub username: String,
    pub email: String,
    pub id: Uuid,
    pub user: UserResponse,
    pub discord_handle: String,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::User,
    DbConn,
};
use leptos::*;

impl AccountResponse {
    pub async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self, ServerFnError> {
        let user = User::find_by_uuid(id, conn).await?;
        let response = UserResponse::from_model(&user, conn).await.map_err(ServerFnError::new)?;
        Ok(Self {
            username: user.username,
            email: user.email,
            id: user.id,
            user: response,
            discord_handle: user.discord_handle.unwrap_or(String::new()),
        })
    }
}
}}
