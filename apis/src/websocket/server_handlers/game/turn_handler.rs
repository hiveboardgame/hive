use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::GameResponse,
    websocket::{
        lag_tracking::{Lags, Pings},
        messages::{InternalServerMessage, MessageDestination},
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, User},
    DbPool,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use hive_lib::{GameError, State, Turn};
use shared_types::{GameId, TimeMode};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

pub struct TurnHandler {
    turn: Turn,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
    pings: actix_web::web::Data<Pings>,
    lags: actix_web::web::Data<Lags>,
}

impl TurnHandler {
    pub fn new(
        turn: Turn,
        game: &Game,
        username: &str,
        user_id: Uuid,
        pings: actix_web::web::Data<Pings>,
        lags: actix_web::web::Data<Lags>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game: game.to_owned(),
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
            turn,
            pings,
            lags,
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        self.users_turn()?;
        let (piece, position) = match self.turn {
            Turn::Move(piece, position) => (piece, position),
            Turn::Shutout => Err(GameError::InvalidTurn {
                username: self.username.to_owned(),
                game: self.game.nanoid.to_owned(),
                turn: format!("{}", self.game.turn),
            })?,
        };
        let mut state = State::new_from_str(&self.game.history, &self.game.game_type)?;
        state.play_turn_from_position(piece, position)?;

        let comp = if self.game.time_mode == TimeMode::RealTime.to_string() {
            let ping = self.pings.value(self.user_id);
            let base = self.game.time_base.unwrap_or(0) as usize;
            let inc = self.game.time_increment.unwrap_or(0) as usize;
            self.lags
                .track_lag(
                    self.user_id,
                    GameId(self.game.nanoid.clone()),
                    ping,
                    base,
                    inc,
                )
                .unwrap_or(0.0)
        } else {
            0.0
        };

        let game = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move { Ok(self.game.update_gamestate(&state, comp, tc).await?) }.scope_boxed()
            })
            .await?;

        match TimeMode::from_str(&game.time_mode) {
            Ok(TimeMode::RealTime) | Err(_) => {}
            _ => {
                let url = format!("http://localhost:8080/msg/{}", game.current_player_id);
                let mut json = HashMap::new();
                let msg = format!(
                    "[It's your turn!](<https://hivegame.com/game/{}>)",
                    game.nanoid
                );
                json.insert("content", msg);
                let client = reqwest::Client::new();
                client.post(url).json(&json).send().await?;
            }
        }

        let mut messages = Vec::new();
        let next_to_move = User::find_by_uuid(&game.current_player_id, &mut conn).await?;
        let games = next_to_move.get_games_with_notifications(&mut conn).await?;
        let mut game_responses = Vec::new();
        for game in games {
            game_responses.push(GameResponse::from_model(&game, &mut conn).await?);
        }
        messages.push(InternalServerMessage {
            destination: MessageDestination::User(game.current_player_id),
            message: ServerMessage::Game(Box::new(GameUpdate::Urgent(game_responses))),
        });
        let response = GameResponse::from_model(&game, &mut conn).await?;
        messages.push(InternalServerMessage {
            destination: MessageDestination::Game(GameId(self.game.nanoid.clone())),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_id: GameId(game.nanoid.to_owned()),
                game: response.clone(),
                game_action: GameReaction::Turn(self.turn.clone()),
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            }))),
        });
        // TODO: Just add the few top games and keep them rated
        if response.time_mode == TimeMode::RealTime {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Global,
                message: ServerMessage::Game(Box::new(GameUpdate::Tv(response))),
            });
        };
        Ok(messages)
    }

    fn users_turn(&self) -> Result<()> {
        if !((self.game.turn % 2 == 0 && self.game.white_id == self.user_id)
            || (self.game.turn % 2 == 1 && self.game.black_id == self.user_id))
        {
            Err(GameError::InvalidTurn {
                username: self.username.to_owned(),
                game: self.game.nanoid.to_owned(),
                turn: format!("{}", self.game.turn),
            })?;
        }
        Ok(())
    }
}
