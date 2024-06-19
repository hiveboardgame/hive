use crate::common::ChallengeAction;
use crate::responses::user::UserResponse;
use chrono::prelude::*;
use hive_lib::{ColorChoice, GameType};
use serde::{Deserialize, Serialize};
use shared_types::{ChallengeDetails, ChallengeId, ChallengeVisibility, GameSpeed, TimeMode};
use std::collections::hash_map::Values;
use std::str;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ChallengeResponse {
    pub id: Uuid,
    pub challenge_id: ChallengeId,
    pub challenger: UserResponse,
    pub opponent: Option<UserResponse>,
    pub game_type: String,
    pub rated: bool,
    pub visibility: ChallengeVisibility,
    pub color_choice: ColorChoice,
    pub created_at: DateTime<Utc>,
    pub challenger_rating: u64,
    pub time_mode: TimeMode,         // Correspondence, Timed, Untimed
    pub time_base: Option<i32>,      // Secons
    pub time_increment: Option<i32>, // Seconds
    pub speed: GameSpeed,
    pub band_upper: Option<i32>,
    pub band_lower: Option<i32>,
}

use cfg_if::cfg_if;
cfg_if! { if #[cfg(feature = "ssr")] {
use db_lib::{
    models::{Challenge, Rating, User},
    DbConn,
};
use anyhow::Result;
impl ChallengeResponse {
    pub async fn from_model(challenge: &Challenge, conn: &mut DbConn<'_>) -> Result<Self> {
        let challenger = challenge.get_challenger(conn).await?;
        ChallengeResponse::from_model_with_user(challenge, challenger, conn).await
    }

    pub async fn from_model_with_user(
        challenge: &Challenge,
        challenger: User,
        conn: &mut DbConn<'_>,
    ) -> Result<Self> {
        let game_speed = GameSpeed::from_base_increment(challenge.time_base, challenge.time_increment);
        let challenger_rating = Rating::for_uuid(&challenger.id, &game_speed, conn).await?;
        let opponent = match challenge.opponent_id {
            None => None,
            Some(id) => Some(UserResponse::from_uuid(&id, conn).await?),
        };
        Ok(ChallengeResponse {
            id: challenge.id,
            challenge_id: ChallengeId(challenge.nanoid.clone()),
            challenger: UserResponse::from_uuid(&challenger.id, conn).await?,
            opponent,
            game_type: challenge.game_type.clone(),
            rated: challenge.rated,
            visibility: ChallengeVisibility::from_str(&challenge.visibility)?,
            color_choice: ColorChoice::from_str(&challenge.color_choice)?,
            created_at: challenge.created_at,
            challenger_rating: challenger_rating.rating as u64,
            time_mode: TimeMode::from_str(&challenge.time_mode)?,
            time_base: challenge.time_base,
            time_increment: challenge.time_increment,
            speed: game_speed,
            band_upper: challenge.band_upper,
            band_lower: challenge.band_lower,
        })
    }

}
}
}

fn is_compatible(challenge: &ChallengeResponse, user: &str, details: &ChallengeDetails) -> bool {
    details.game_type == GameType::from_str(&challenge.game_type).unwrap()
        && details.rated == challenge.rated
        && details.time_mode == challenge.time_mode
        && details.time_base == challenge.time_base
        && details.time_increment == challenge.time_increment
        && match details.color_choice {
            ColorChoice::Random => challenge.color_choice == ColorChoice::Random,
            ColorChoice::White => challenge.color_choice == ColorChoice::Black,
            ColorChoice::Black => challenge.color_choice == ColorChoice::White,
        }
        && if let Some(opponent) = &details.opponent {
            opponent == &challenge.challenger.username
        } else if let Some(opponent) = &challenge.opponent {
            opponent.username == user
        } else {
            challenge.challenger.username != user
        }
}

fn has_same_details(challenge: &ChallengeResponse, user: &str, details: &ChallengeDetails) -> bool {
    let challenge_opponent = challenge
        .opponent
        .as_ref()
        .map(|opponent| opponent.username.as_str());

    details.game_type == GameType::from_str(&challenge.game_type).unwrap()
        && details.rated == challenge.rated
        && details.band_lower == challenge.band_upper
        && details.band_upper == challenge.band_lower
        && details.time_mode == challenge.time_mode
        && details.time_base == challenge.time_base
        && details.time_increment == challenge.time_increment
        && details.color_choice == challenge.color_choice
        && challenge_opponent == details.opponent.as_deref()
        && challenge.challenger.username == user
}

pub fn create_challenge_handler(
    user: String,
    details: ChallengeDetails,
    mut challenges: Values<ChallengeId, ChallengeResponse>,
) -> Option<ChallengeAction> {
    if details.time_mode == TimeMode::RealTime
        && challenges.any(|c| has_same_details(c, &user, &details))
    {
        None
    } else if let Some(challenge) = challenges.find(|c| is_compatible(c, &user, &details)) {
        Some(ChallengeAction::Accept(challenge.challenge_id.clone()))
    } else {
        Some(ChallengeAction::Create(details))
    }
}
