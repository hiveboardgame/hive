use crate::{
    common::{ChallengeUpdate, GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::GameResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    models::{Challenge, Game, NewGame, Rating},
    DbPool,
};
use shared_types::GameSpeed;
use uuid::Uuid;

pub struct AcceptHandler {
    nanoid: String,
    user_id: Uuid,
    username: String,
    pool: DbPool,
}

impl AcceptHandler {
    pub async fn new(nanoid: String, username: &str, user_id: Uuid, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            nanoid,
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut messages = Vec::new();
        let challenge = Challenge::find_by_nanoid(&self.nanoid, &self.pool).await?;
        let speed = GameSpeed::from_base_increment(challenge.time_base, challenge.time_increment);
        let rating = Rating::for_uuid(&self.user_id, &speed, &self.pool)
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
                        "{rating} is above the rating band of {band_lower}"
                    )),
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

        let new_game = NewGame::new(white_id, black_id, &challenge);
        let (game, deleted_challenges) = Game::create(&new_game, &self.pool).await?;
        let game_response = GameResponse::new_from_db(&game, &self.pool).await?;

        messages.push(InternalServerMessage {
            destination: MessageDestination::User(game.white_id),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_action: GameReaction::New,
                game: game_response.clone(),
                game_id: game_response.nanoid.clone(),
                user_id: self.user_id,
                username: self.username.to_owned(),
            }))),
        });

        messages.push(InternalServerMessage {
            destination: MessageDestination::User(game.black_id),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_action: GameReaction::New,
                game: game_response.clone(),
                game_id: game_response.nanoid.clone(),
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
