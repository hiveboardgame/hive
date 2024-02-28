use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AccountResponse {
    pub username: String,
    pub email: String,
    pub id: Uuid,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::user::User,
    DbPool,
};
use leptos::*;
impl AccountResponse {
    pub async fn from_uuid(id: &Uuid, pool: &DbPool) -> Result<Self, ServerFnError> {
        let user = User::find_by_uuid(id, pool).await?;
        Ok(Self {
            username: user.username,
            email: user.email,
            id: user.id,
        })
    }
}
}}
