use crate::common::ChallengeVisibility;
use crate::common::{ChallengeUpdate, ServerMessage};
use crate::responses::ChallengeResponse;
use crate::websockets::internal_server_message::{InternalServerMessage, MessageDestination};
use anyhow::Result;
use db_lib::{
    models::{Challenge, NewChallenge,User},
    DbPool,
};
use hive_lib::{color::ColorChoice, game_type::GameType};
use shared_types::TimeMode;
use std::str::FromStr;
use uuid::Uuid;

pub struct CreateHandler {
    rated: bool,
    game_type: GameType,
    visibility: ChallengeVisibility,
    opponent: Option<String>,
    color_choice: ColorChoice,
    time_mode: TimeMode,
    time_base: Option<i32>,
    time_increment: Option<i32>,
    band_upper: Option<i32>,
    band_lower: Option<i32>,
    user_id: Uuid,
    pool: DbPool,
}

impl CreateHandler {
    pub async fn new(
        rated: bool,
        game_type: GameType,
        visibility: ChallengeVisibility,
        color_choice: ColorChoice,
        opponent: Option<String>,
        time_mode: TimeMode,
        time_base: Option<i32>,
        time_increment: Option<i32>,
        band_upper: Option<i32>,
        band_lower: Option<i32>,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            rated,
            game_type,
            visibility,
            color_choice,
            opponent,
            time_mode,
            time_base,
            time_increment,
            band_upper,
            band_lower,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let opponent = match &self.opponent {
            Some(username) => Some((User::find_by_username(username, &self.pool).await?).id),
            None => None,
        };

        let new_challenge = NewChallenge::new(
            self.user_id,
            opponent,
            self.game_type,
            self.rated,
            self.visibility.to_string(),
            self.color_choice.to_string(),
            self.time_mode.clone(),
            self.time_base,
            self.time_increment,
            self.band_upper,
            self.band_lower,
        )?;

        let challenge = Challenge::create(&new_challenge, &self.pool).await?;
        let challenge_response = ChallengeResponse::from_model(&challenge, &self.pool).await?;
        let mut messages = Vec::new();
        match ChallengeVisibility::from_str(&new_challenge.visibility)? {
            ChallengeVisibility::Direct => {
                if let Some(ref opponent) = challenge_response.opponent {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(opponent.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                            challenge_response.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::User(challenge_response.challenger.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                            challenge_response,
                        )),
                    });
                }
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(challenge_response.challenger.uid),
                    message: ServerMessage::Challenge(ChallengeUpdate::Direct(challenge_response)),
                });
            }
            ChallengeVisibility::Public => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Global,
                    message: ServerMessage::Challenge(ChallengeUpdate::Created(challenge_response)),
                });
            }
        }
        Ok(messages)
    }
}
