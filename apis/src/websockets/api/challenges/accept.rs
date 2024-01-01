use crate::{
    common::{
        challenge_action::ChallengeVisibility,
        server_result::{
            ChallengeUpdate, InternalServerMessage, MessageDestination, ServerMessage,
        },
    },
    responses::game::GameResponse,
};
use anyhow::Result;
use db_lib::{
    models::challenge::Challenge,
    models::game::{Game, NewGame},
    DbPool,
};
use std::str::FromStr;
use uuid::Uuid;

pub struct AcceptHandler {
    nanoid: String,
    user_id: Uuid,
    pool: DbPool,
}

impl AcceptHandler {
    pub async fn new(nanoid: String, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let challenge = Challenge::find_by_nanoid(&self.nanoid, &self.pool).await?;
        let (white_id, black_id) = match challenge.color_choice.to_lowercase().as_str() {
            "black" => (self.user_id, challenge.challenger_id),
            "white" => (challenge.challenger_id, self.user_id),
            _ => {
                if rand::random() {
                    (challenge.challenger_id, self.user_id)
                } else {
                    (self.user_id, challenge.challenger_id)
                }
            }
        };

        let new_game = NewGame::new(white_id, black_id, &challenge);
        let game = Game::create(&new_game, &self.pool).await?;
        challenge.delete(&self.pool).await?;
        let mut messages = Vec::new();
        let game_response = GameResponse::new_from_db(&game, &self.pool).await?;

        messages.push(InternalServerMessage {
            destination: MessageDestination::Direct(game.white_id),
            message: ServerMessage::GameNew(game_response.clone()),
        });

        messages.push(InternalServerMessage {
            destination: MessageDestination::Direct(game.black_id),
            message: ServerMessage::GameNew(game_response),
        });

        match ChallengeVisibility::from_str(&challenge.visibility)? {
            ChallengeVisibility::Public => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Global,
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge.nanoid)),
                });
            }
            ChallengeVisibility::Private => {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Direct(challenge.challenger_id),
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge.nanoid)),
                });
            }
            ChallengeVisibility::Direct => {
                if let Some(opponent) = challenge.opponent_id {
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::Direct(opponent),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge.nanoid.clone(),
                        )),
                    });
                    messages.push(InternalServerMessage {
                        destination: MessageDestination::Direct(challenge.challenger_id),
                        message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                            challenge.nanoid,
                        )),
                    });
                }
            }
        }
        Ok(messages)
    }
}
