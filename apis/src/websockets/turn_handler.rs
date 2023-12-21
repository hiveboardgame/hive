use crate::{
    common::{
        game_action::GameAction,
        server_result::{
            GameActionResponse, InternalServerMessage, MessageDestination, ServerMessage,
        },
    },
    functions::games::game_response::GameStateResponse,
};
use anyhow::Result;
use db_lib::{models::game::Game, models::user::User, DbPool};
use hive_lib::{game_error::GameError, state::State, turn::Turn};
use uuid::Uuid;

pub struct TurnHandler {
    turn: Turn,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl TurnHandler {
    pub async fn new(
        turn: Turn,
        game: Game,
        username: &str,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            game,
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
            turn,
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut messages = Vec::new();
        self.users_turn()?;
        let (piece, position) = match self.turn {
            Turn::Move(piece, position) => (piece, position),
            Turn::Spawn(piece, position) => (piece, position),
            Turn::Shutout => Err(GameError::InvalidTurn {
                username: self.username.to_owned(),
                game: self.game.nanoid.to_owned(),
                turn: format!("{}", self.game.turn),
            })?,
        };

        let mut state = State::new_from_str(&self.game.history, &self.game.game_type)?;
        let current_turn = state.turn;
        state.play_turn_from_position(piece, position)?;
        let (piece, pos) = state
            .history
            .moves
            .get(current_turn)
            .expect("No moves in history after a move has been played.");
        // TODO: @leex making 2 DB inserts is a bit ugly, maybe we should have:
        // make_move and make_moves?
        let mut game = self
            .game
            .make_move(
                format!("{piece} {pos}"),
                state.game_status.clone(),
                &self.pool,
            )
            .await?;
        if state
            .history
            .moves
            .last()
            .expect("There needs to be a move here")
            .0
            == "pass"
        {
            game = self
                .game
                .make_move(String::from("pass "), state.game_status.clone(), &self.pool)
                .await?;
        }
        let current_user = User::find_by_uuid(&game.current_player_id, &self.pool).await?;
        let game_ids = current_user
            .get_games_with_notifications(&self.pool)
            .await?
            .into_iter()
            .map(|game| game.nanoid)
            .collect();
        messages.push(InternalServerMessage {
            destination: MessageDestination::Direct(game.current_player_id),
            message: ServerMessage::GameActionNotification(game_ids),
        });
        messages.push(InternalServerMessage {
            destination: MessageDestination::Game(self.game.nanoid.clone()),
            message: ServerMessage::GameUpdate(GameActionResponse {
                game_id: self.game.nanoid.to_owned(),
                game: GameStateResponse::new_from_db(&game, &self.pool).await?,
                game_action: GameAction::Move(self.turn.clone()),
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            }),
        });
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
