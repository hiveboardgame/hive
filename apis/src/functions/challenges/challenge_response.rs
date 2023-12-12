use chrono::prelude::*;
use hive_lib::color::ColorChoice;
use serde::{Deserialize, Serialize};
use std::str;
use thiserror::Error;
use uuid::Uuid;

use crate::functions::users::user_response::UserResponse;

#[derive(Clone, Error, Debug, Deserialize, Serialize)]
pub enum ChallengeError {
    #[error("Couldn't find challenge creator (uid {0})")]
    MissingChallenger(String),
    #[error("You can't accept your own challenges!")]
    OwnChallenge,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChallengeResponse {
    pub id: Uuid,
    pub nanoid: String,
    pub challenger: UserResponse,
    pub game_type: String,
    pub rated: bool,
    pub public: bool,
    pub tournament_queen_rule: bool,
    pub color_choice: ColorChoice,
    pub created_at: DateTime<Utc>,
    pub challenger_rating: f64,
}

#[cfg(feature = "ssr")]
use db_lib::{
    models::{challenge::Challenge, user::User},
    DbPool,
};
#[cfg(feature = "ssr")]
use leptos::*;
#[cfg(feature = "ssr")]
impl ChallengeResponse {
    pub async fn from_model(challenge: &Challenge, pool: &DbPool) -> Result<Self, ServerFnError> {
        let challenger = challenge.get_challenger(pool).await?;
        ChallengeResponse::from_model_with_user(challenge, challenger, pool).await
    }

    pub async fn from_model_with_user(
        challenge: &Challenge,
        challenger: User,
        pool: &DbPool,
    ) -> Result<Self, ServerFnError> {
        use db_lib::models::rating::Rating;
        use std::str::FromStr;
        let challenger_rating = Rating::for_uuid(&challenger.id, pool).await?;
        Ok(ChallengeResponse {
            id: challenge.id,
            nanoid: challenge.nanoid.to_owned(),
            challenger: UserResponse::from_uuid(&challenger.id, pool)
                .await
                .map_err(|e| ServerFnError::ServerError(e.to_string()))?,
            game_type: challenge.game_type.clone(),
            rated: challenge.rated,
            public: challenge.public,
            tournament_queen_rule: challenge.tournament_queen_rule,
            color_choice: ColorChoice::from_str(&challenge.color_choice)?,
            created_at: challenge.created_at,
            challenger_rating: challenger_rating.rating,
        })
    }
}
