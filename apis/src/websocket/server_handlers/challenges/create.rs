use crate::{
    common::{ChallengeUpdate, ServerMessage},
    notifications::{notify, time_control_label, Event},
    responses::ChallengeResponse,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        WebsocketData,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Challenge, NewChallenge, User},
    DbPool,
};
use shared_types::{ChallengeDetails, ChallengeVisibility, TimeMode};
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

pub struct CreateHandler {
    details: ChallengeDetails,
    user_id: Uuid,
    pool: DbPool,
    data: Arc<WebsocketData>,
}

impl CreateHandler {
    pub async fn new(
        details: ChallengeDetails,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            details,
            user_id,
            pool: pool.clone(),
            data,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let opponent = match &self.details.opponent {
            Some(username) => Some((User::find_by_username(username, &mut conn).await?).id),
            None => None,
        };

        let new_challenge =
            NewChallenge::new(self.user_id, opponent, &self.details, &mut conn).await?;
        let challenge = self
            .data
            .realtime_gate
            .with_realtime_admission(
                self.details.time_mode == TimeMode::RealTime,
                Challenge::create(&new_challenge, &mut conn),
            )
            .await?;
        let challenge_response = ChallengeResponse::from_model(&challenge, &mut conn).await?;
        let mut messages = Vec::new();
        match ChallengeVisibility::from_str(&new_challenge.visibility)? {
            ChallengeVisibility::Direct => {
                if let Some(ref opponent) = challenge_response.opponent {
                    notify(Event::ChallengeReceived {
                        recipient: opponent.uid,
                        challenger: challenge_response.challenger.username.clone(),
                        challenge_nanoid: challenge_response.challenge_id.0.clone(),
                        time_control: time_control_label(
                            challenge_response.speed,
                            challenge_response.time_base,
                            challenge_response.time_increment,
                        ),
                        rated: challenge_response.rated,
                    });
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
