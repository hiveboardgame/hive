use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    responses::GameResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, Tournament, User},
    DbPool,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use hive_lib::{GameError, State, Turn};
use shared_types::{GameId, TimeMode};
use std::str::FromStr;
use uuid::Uuid;

pub struct TurnHandler {
    turn: Turn,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl TurnHandler {
    pub fn new(turn: Turn, game: &Game, username: &str, user_id: Uuid, pool: &DbPool) -> Self {
        Self {
            game: game.to_owned(),
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
            turn,
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
            //let ping = self.data.pings.value(self.user_id);
            let base = self.game.time_base.unwrap_or(0) as usize;
            let inc = self.game.time_increment.unwrap_or(0) as usize;
            /*self.data
            .lags
            .track_lag(
                self.user_id,
                GameId(self.game.nanoid.clone()),
                ping,
                base,
                inc,
            )
            .unwrap_or(0.0)*/
            0.0
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
                let opponent_id = game.not_current_player_id();
                let opponent = User::find_by_uuid(&opponent_id, &mut conn).await?;
                let tournament_name = if let Some(id) = game.tournament_id {
                    let tournament = Tournament::find_by_uuid(id, &mut conn).await?;
                    format!(" (Tournament: {})", tournament.name)
                } else {
                    String::new()
                };

                let msg = format!("[Your turn](<https://hivegame.com/game/{}>) in your game vs {}{}.\nYou have {} to play.",
                    game.nanoid,
                    opponent.username,
                    tournament_name,
                    game.str_time_left_for_player(game.current_player_id),
                );

                /*if let Err(e) = Busybee::msg(game.current_player_id, msg).await {
                    println!("{e}");
                };*/
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
        // TODO: refactor to self.game.current_player_id == self.user_id
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
