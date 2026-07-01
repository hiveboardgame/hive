use crate::{common::ChallengeAction, responses::user::UserResponse};
use chrono::prelude::*;
use hudsoni::{ColorChoice, GameType};
use serde::{Deserialize, Serialize};
use shared_types::{ChallengeDetails, ChallengeId, ChallengeVisibility, GameSpeed, TimeMode};
use std::{str, str::FromStr};
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
    models::Challenge,
    DbConn,
};
use anyhow::Result;
use std::collections::HashSet;
impl ChallengeResponse {
    pub async fn from_model(challenge: &Challenge, conn: &mut DbConn<'_>) -> Result<Self> {
        let challenger = UserResponse::from_uuid(&challenge.challenger_id, conn).await?;
        let opponent = match challenge.opponent_id {
            Some(opponent_id) => Some(UserResponse::from_uuid(&opponent_id, conn).await?),
            None => None,
        };
        Self::from_model_parts(challenge, challenger, opponent)
    }

    pub async fn from_models_batch(
        challenges: Vec<Challenge>,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>> {
        if challenges.is_empty() {
            return Ok(Vec::new());
        }

        let mut user_ids = HashSet::new();
        for challenge in &challenges {
            user_ids.insert(challenge.challenger_id);
            if let Some(opponent_id) = challenge.opponent_id {
                user_ids.insert(opponent_id);
            }
        }

        let user_ids: Vec<Uuid> = user_ids.into_iter().collect();
        let users = UserResponse::from_uuids(&user_ids, conn).await?;

        let mut responses = Vec::with_capacity(challenges.len());
        for challenge in challenges {
            let challenger = users.get(&challenge.challenger_id).cloned().ok_or_else(|| {
                anyhow::anyhow!(
                    "Challenger {} not found for challenge {}",
                    challenge.challenger_id,
                    challenge.id
                )
            })?;
            let opponent = match challenge.opponent_id {
                Some(opponent_id) => Some(users.get(&opponent_id).cloned().ok_or_else(|| {
                    anyhow::anyhow!(
                        "Opponent {} not found for challenge {}",
                        opponent_id,
                        challenge.id
                    )
                })?),
                None => None,
            };
            responses.push(Self::from_model_parts(&challenge, challenger, opponent)?);
        }

        Ok(responses)
    }

    fn from_model_parts(
        challenge: &Challenge,
        challenger: UserResponse,
        opponent: Option<UserResponse>,
    ) -> Result<Self> {
        let game_speed =
            GameSpeed::from_base_increment(challenge.time_base, challenge.time_increment);
        let challenger_rating = challenger.rating_for_speed(&game_speed);
        Ok(ChallengeResponse {
            id: challenge.id,
            challenge_id: ChallengeId(challenge.nanoid.clone()),
            challenger,
            opponent,
            game_type: challenge.game_type.clone(),
            rated: challenge.rated,
            visibility: ChallengeVisibility::from_str(&challenge.visibility)?,
            color_choice: ColorChoice::from_str(&challenge.color_choice)?,
            created_at: challenge.created_at,
            challenger_rating,
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

fn is_compatible(
    existing_challenge: &ChallengeResponse,
    new_challenge_details: &ChallengeDetails,
    challenger_name: &str,
) -> bool {
    let opponent_matches = new_challenge_details
        .opponent
        .as_ref()
        .is_none_or(|opponent_name| existing_challenge.challenger.username == *opponent_name);

    opponent_matches
        && !existing_challenge.challenger.bot
        && new_challenge_details.game_type
            == GameType::from_str(&existing_challenge.game_type).unwrap()
        && new_challenge_details.rated == existing_challenge.rated
        && new_challenge_details.time_mode == existing_challenge.time_mode
        && new_challenge_details.time_base == existing_challenge.time_base
        && new_challenge_details.time_increment == existing_challenge.time_increment
        && match new_challenge_details.color_choice {
            ColorChoice::Random => existing_challenge.color_choice == ColorChoice::Random,
            ColorChoice::White => existing_challenge.color_choice == ColorChoice::Black,
            ColorChoice::Black => existing_challenge.color_choice == ColorChoice::White,
        }
        && existing_challenge.challenger.username != challenger_name
        && (
            new_challenge_details.band_lower,
            new_challenge_details.band_upper,
        ) == (None, None)
        && (existing_challenge.band_lower, existing_challenge.band_upper) == (None, None)
}

fn has_same_details(
    existing_challenge: &ChallengeResponse,
    new_challenge_details: &ChallengeDetails,
    challenger_name: &str,
) -> bool {
    let challenge_opponent = existing_challenge
        .opponent
        .as_ref()
        .map(|opponent| opponent.username.as_str());

    new_challenge_details.game_type == GameType::from_str(&existing_challenge.game_type).unwrap()
        && new_challenge_details.rated == existing_challenge.rated
        && new_challenge_details.band_lower == existing_challenge.band_upper
        && new_challenge_details.band_upper == existing_challenge.band_lower
        && new_challenge_details.time_mode == existing_challenge.time_mode
        && new_challenge_details.time_base == existing_challenge.time_base
        && new_challenge_details.time_increment == existing_challenge.time_increment
        && new_challenge_details.color_choice == existing_challenge.color_choice
        && challenge_opponent == new_challenge_details.opponent.as_deref()
        && existing_challenge.challenger.username == challenger_name
}

//TODO: Move this code that only gets used in the frontend
pub fn create_challenge_handler(
    challenger_name: String,
    new_challenge_details: ChallengeDetails,
    challenges: Vec<ChallengeResponse>,
) -> Option<ChallengeAction> {
    if new_challenge_details.time_mode == TimeMode::RealTime
        && challenges.iter().any(|existing_challenge| {
            has_same_details(existing_challenge, &new_challenge_details, &challenger_name)
        })
    {
        None
    } else if let Some(existing_challenge) = &challenges.iter().find(|existing_challenge| {
        is_compatible(existing_challenge, &new_challenge_details, &challenger_name)
    }) {
        Some(ChallengeAction::Accept(
            existing_challenge.challenge_id.clone(),
        ))
    } else {
        Some(ChallengeAction::Create(new_challenge_details))
    }
}
