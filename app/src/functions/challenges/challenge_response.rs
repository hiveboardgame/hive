use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str;
use thiserror::Error;

use crate::functions::users::user_response::UserResponse;

#[derive(Clone, Error, Debug)]
pub enum ChallengeError {
    #[error("Couldn't find challenge creator (uid {0})")]
    MissingChallenger(String),
    #[error("You can't accept your own challenges!")]
    OwnChallenge,
    #[error("\"(0)\" is not a valid color choice string")]
    ColorChoiceError(String),
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum ColorChoice {
    White,
    Black,
    Random,
}

impl fmt::Display for ColorChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::White => write!(f, "White"),
            Self::Black => write!(f, "Black"),
            Self::Random => write!(f, "Random"),
        }
    }
}

impl str::FromStr for ColorChoice {
    type Err = ChallengeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "White" => Ok(ColorChoice::White),
            "Black" => Ok(ColorChoice::Black),
            "Random" => Ok(ColorChoice::Random),
            _ => Err(ChallengeError::ColorChoiceError(s.to_string())),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChallengeResponse {
    pub id: String, //Uuid
    pub challenger: UserResponse,
    pub game_type: String,
    pub rated: bool,
    pub public: bool,
    pub tournament_queen_rule: bool,
    pub color_choice: String,
    pub created_at: DateTime<Utc>,
    pub challenger_rating: f64,
}

#[cfg(feature = "ssr")]
use db_lib::{
    models::{challenge::Challenge, rating::Rating, user::User},
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
        let challenger_rating = Rating::for_uid(&challenger.uid, pool).await?;
        Ok(ChallengeResponse {
            id: challenge.id.to_string(),
            challenger: UserResponse::from_uid(&challenger.uid, pool).await?,
            game_type: challenge.game_type.clone(),
            rated: challenge.rated,
            public: challenge.public,
            tournament_queen_rule: challenge.tournament_queen_rule,
            color_choice: challenge.color_choice.clone(),
            created_at: challenge.created_at,
            challenger_rating: challenger_rating.rating,
        })
    }
}
