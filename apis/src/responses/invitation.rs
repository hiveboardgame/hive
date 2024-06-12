use super::{TournamentResponse, UserResponse};
use serde::{Deserialize, Serialize};
use std::str;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InvitationResponse {
    pub tournament: TournamentResponse,
    pub invitee: UserResponse,
}

cfg_if::cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{User, Tournament},
    DbPool,
};

use anyhow::Result;
impl InvitationResponse {
    // pub async fn from_model(tournament: &Tournament, invitee: &User, pool: &DbPool) -> Result<Self> {
    //     InvitationResponse::from_model_with_user(tournament, invitee, pool).await
    // }

    pub async fn from_models(
        tournament: &Tournament,
        invitee: &User,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(InvitationResponse {
            tournament: TournamentResponse::from_model(tournament, pool).await?,
            invitee: UserResponse::from_model(invitee, pool).await?,
        })
    }
}

}}
