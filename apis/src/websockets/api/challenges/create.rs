use crate::common::challenge_action::ChallengeVisibility;
use crate::common::server_result::{
    ChallengeUpdate, InternalServerMessage, MessageDestination, ServerMessage,
};
use crate::responses::challenge::ChallengeResponse;
use anyhow::Result;
use db_lib::{
    models::challenge::{Challenge, NewChallenge},
    models::user::User,
    DbPool,
};
use hive_lib::{color::ColorChoice, game_type::GameType};
use std::str::FromStr;
use uuid::Uuid;

pub struct CreateHandler {
    rated: bool,
    game_type: GameType,
    visibility: ChallengeVisibility,
    opponent: Option<String>,
    color_choice: ColorChoice,
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
        user_id: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            rated,
            game_type,
            visibility,
            color_choice,
            opponent,
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
        );

        let challenge = Challenge::create(&new_challenge, &self.pool).await?;
        let challenge_response = ChallengeResponse::from_model(&challenge, &self.pool).await?;
        let mut messages = Vec::new();
        match ChallengeVisibility::from_str(&new_challenge.visibility)? {
            ChallengeVisibility::Direct => {
                if let Some(ref opponent) = challenge_response.opponent {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::Direct(opponent.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                            challenge_response.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::Direct(challenge_response.challenger.uid),
                        message: ServerMessage::Challenge(ChallengeUpdate::Direct(
                            challenge_response,
                        )),
                    });
                }
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Direct(challenge_response.challenger.uid),
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
