use crate::{
    common::{ChallengeUpdate, ServerMessage, UiError},
    responses::GameResponse,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        server_handlers::game::new_game_messages,
        RealtimeGate,
    },
};
use anyhow::Result;
use db_lib::{
    db_error::DbError,
    get_conn,
    models::{Challenge, Game, NewGame, Rating},
    DbPool,
};
use diesel_async::AsyncConnection;
use shared_types::{rounded_rating, ChallengeId, GameSpeed};
use std::sync::Arc;
use uuid::Uuid;

pub struct AcceptHandler {
    challenge_id: ChallengeId,
    user_id: Uuid,
    pool: DbPool,
}

impl AcceptHandler {
    pub fn new(challenge_id: ChallengeId, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            challenge_id,
            user_id,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self, gate: &RealtimeGate) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let mut messages = Vec::new();
        let challenge = match Challenge::find_by_challenge_id(&self.challenge_id, &mut conn).await {
            Ok(challenge) => challenge,
            Err(DbError::NotFound { .. }) => {
                return Ok(vec![InternalServerMessage {
                    destination: MessageDestination::User(self.user_id),
                    message: ServerMessage::Challenge(ChallengeUpdate::Removed(
                        self.challenge_id.clone(),
                    )),
                }]);
            }
            Err(err) => return Err(err.into()),
        };
        challenge.validate_accepting_user(self.user_id)?;
        gate.admit_challenge(&challenge)?;
        let speed = challenge
            .time_control()?
            .map_or(GameSpeed::Untimed, GameSpeed::from);
        let rating = rounded_rating(
            Rating::for_uuid(&self.user_id, &speed, &mut conn)
                .await?
                .rating,
        )
        .unwrap_or_default();
        if let Some(band_upper) = challenge.band_upper {
            if i64::from(rating) > i64::from(band_upper) {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(self.user_id),
                    message: ServerMessage::Error(
                        UiError::RatingAboveBand {
                            rating: rating.to_string(),
                            limit: band_upper.to_string(),
                        }
                        .encode(),
                    ),
                });
                return Ok(messages);
            }
        }
        if let Some(band_lower) = challenge.band_lower {
            if i64::from(rating) < i64::from(band_lower) {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(self.user_id),
                    message: ServerMessage::Error(
                        UiError::RatingBelowBand {
                            rating: rating.to_string(),
                            limit: band_lower.to_string(),
                        }
                        .encode(),
                    ),
                });
                return Ok(messages);
            }
        }
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

        let (deleted_challenges, game_response) = conn
            .transaction::<_, anyhow::Error, _>(async move |tc| {
                let new_game = NewGame::new(white_id, black_id, &challenge)?;
                let (game, deleted_challenges) =
                    Game::create_and_delete_challenges(new_game, tc).await?;
                let game_response = Arc::new(GameResponse::from_model(&game, tc).await?);
                Ok((deleted_challenges, game_response))
            })
            .await?;

        messages.extend(new_game_messages(game_response, Some(self.user_id), None));

        for challenge_nanoid in deleted_challenges {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge_nanoid)),
            });
        }
        Ok(messages)
    }
}
