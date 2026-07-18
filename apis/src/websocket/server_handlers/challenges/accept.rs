use crate::{
    common::{ChallengeUpdate, GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    notifications::{notify, time_control_label, Event},
    responses::GameResponse,
    websocket::{
        messages::{InternalServerMessage, MessageDestination},
        WebsocketData,
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
use shared_types::{ChallengeId, GameSpeed, TimeMode};
use std::sync::Arc;
use uuid::Uuid;

pub struct AcceptHandler {
    challenge_id: ChallengeId,
    user_id: Uuid,
    username: String,
    pool: DbPool,
    data: Arc<WebsocketData>,
}

impl AcceptHandler {
    pub async fn new(
        challenge_id: ChallengeId,
        username: &str,
        user_id: Uuid,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            challenge_id,
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
            data,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
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
        let time_mode = challenge.parsed_time_mode()?;
        challenge.validate_accepting_user(self.user_id)?;
        let speed = GameSpeed::from_base_increment(challenge.time_base, challenge.time_increment);
        let rating = Rating::for_uuid(&self.user_id, &speed, &mut conn)
            .await?
            .rating;
        if let Some(band_upper) = challenge.band_upper {
            if rating > band_upper as f64 {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(self.user_id),
                    message: ServerMessage::Error(format!(
                        "{rating} is above the rating band of {band_upper}"
                    )),
                });
                return Ok(messages);
            }
        }
        if let Some(band_lower) = challenge.band_lower {
            if rating < band_lower as f64 {
                messages.push(InternalServerMessage {
                    destination: MessageDestination::User(self.user_id),
                    message: ServerMessage::Error(format!(
                        "{rating} is below the rating band of {band_lower}"
                    )),
                });
                return Ok(messages);
            }
        }
        let challenger_id = challenge.challenger_id;
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

        let (game, deleted_challenges, game_response) = self
            .data
            .realtime_gate
            .with_realtime_admission(time_mode == TimeMode::RealTime, async {
                conn.transaction::<_, anyhow::Error, _>(async move |tc| {
                    let new_game = NewGame::new(white_id, black_id, &challenge)?;
                    let (game, deleted_challenges) =
                        Game::create_and_delete_challenges(new_game, tc).await?;
                    let game_response = GameResponse::from_model(&game, tc).await?;
                    Ok((game, deleted_challenges, game_response))
                })
                .await
            })
            .await?;

        notify(Event::GameStarted {
            recipient: challenger_id,
            opponent: self.username.clone(),
            game_nanoid: game.nanoid.clone(),
            time_control: time_control_label(speed, game.time_base, game.time_increment),
            speed,
        });

        messages.push(InternalServerMessage {
            destination: MessageDestination::User(game.white_id),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_action: GameReaction::New,
                game: game_response.clone(),
                game_id: game_response.game_id.clone(),
                user_id: self.user_id,
                username: self.username.to_owned(),
            }))),
        });

        messages.push(InternalServerMessage {
            destination: MessageDestination::User(game.black_id),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_action: GameReaction::New,
                game: game_response.clone(),
                game_id: game_response.game_id.clone(),
                user_id: self.user_id,
                username: self.username.to_owned(),
            }))),
        });

        for challenge_nanoid in deleted_challenges {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Challenge(ChallengeUpdate::Removed(challenge_nanoid)),
            });
        }
        Ok(messages)
    }
}
