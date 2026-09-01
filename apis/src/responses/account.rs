use crate::responses::UserResponse;
use serde::{Deserialize, Serialize};
use shared_types::TimeWarning;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AccountResponse {
    pub username: String,
    pub email: String,
    pub id: Uuid,
    pub user: UserResponse,
    pub time_warnings: Vec<TimeWarning>,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::User,
    DbConn,
};
use leptos::prelude::*;

impl AccountResponse {
    pub async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self, ServerFnError> {
        let user = User::find_active_by_uuid(id, conn).await?;
        let response = UserResponse::from_model(&user, conn).await.map_err(ServerFnError::new)?;
        let time_warnings = user
            .time_warning_stages()
            .unwrap_or_else(shared_types::default_time_warnings);
        Ok(Self {
            username: user.username,
            email: user.email,
            id: user.id,
            user: response,
            time_warnings,
        })
    }
}
}}
