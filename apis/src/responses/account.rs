use crate::responses::UserResponse;
use serde::{Deserialize, Serialize};
use shared_types::GameSpeed;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AccountResponse {
    pub username: String,
    pub email: String,
    pub id: Uuid,
    pub user: UserResponse,
    pub admission_ratings: HashMap<GameSpeed, f64>,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{Rating, User},
    DbConn,
};
use leptos::prelude::*;
use std::str::FromStr;

impl AccountResponse {
    pub async fn from_uuid(id: &Uuid, conn: &mut DbConn<'_>) -> Result<Self, ServerFnError> {
        let user = User::find_active_by_uuid(id, conn).await?;
        let ratings = Rating::for_uuids(&[*id], conn).await?;
        let response = UserResponse::from_models_with_ratings(std::slice::from_ref(&user), &ratings)
            .map_err(ServerFnError::new)?.remove(id)
            .ok_or_else(|| ServerFnError::new("Account user response was not assembled"))?;
        let admission_ratings = ratings
            .into_iter()
            .map(|rating| Ok((GameSpeed::from_str(&rating.speed).map_err(ServerFnError::new)?, rating.rating)))
            .collect::<Result<HashMap<_, _>, ServerFnError>>()?;
        Ok(Self {
            username: user.username,
            email: user.email,
            id: user.id,
            user: response,
            admission_ratings,
        })
    }
}
}}
