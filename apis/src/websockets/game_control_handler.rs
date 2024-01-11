use crate::{
    common::{
        game_action::GameAction,
        server_result::{
            GameActionResponse, InternalServerMessage, MessageDestination, ServerMessage,
        },
    },
    responses::game::GameResponse,
};
use anyhow::Result;
use db_lib::{models::game::Game, models::user::User, DbPool};
use hive_lib::{game_control::GameControl, game_error::GameError};
use uuid::Uuid;

pub struct GameControlHandler {
    control: GameControl,
    pool: DbPool,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl GameControlHandler {
    pub async fn new(
        control: &GameControl,
        game: Game,
        username: &str,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Self {
        Self {
            game,
            user_id,
            username: username.to_owned(),
            pool: pool.clone(),
            control: control.to_owned(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut messages = Vec::new();
        // the GC can be played this turn
        self.ensure_gc_allowed_for_turn()?;
        // the GC(color) matches the user color
        self.ensure_gc_color()?;
        // the gc hasn't been sent previously
        self.ensure_fresh_game_control()?;
        let game = self.match_control().await?;
        messages.push(InternalServerMessage {
            destination: MessageDestination::Game(self.game.nanoid.clone()),
            message: ServerMessage::GameUpdate(GameActionResponse {
                game_id: self.game.nanoid.to_owned(),
                game: GameResponse::new_from_db(&game, &self.pool).await?,
                game_action: GameAction::Control(self.control.clone()),
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            }),
        });
        match self.control {
            GameControl::DrawOffer(_) | GameControl::TakebackRequest(_) => {
                let current_user = User::find_by_uuid(&game.current_player_id, &self.pool).await?;
                let games = current_user
                    .get_games_with_notifications(&self.pool)
                    .await?;
                let mut game_responses = Vec::new();
                for game in games {
                    game_responses.push(GameResponse::new_from_db(&game, &self.pool).await?);
                }
                messages.push(InternalServerMessage {
                    destination: MessageDestination::Direct(game.current_player_id),
                    message: ServerMessage::GameActionNotification(game_responses),
                });
            }
            _ => {}
        }
        Ok(messages)
    }

    fn ensure_fresh_game_control(&self) -> Result<()> {
        if let Some(last) = self.game.last_game_control() {
            if last == self.control {
                Err(GameError::GcAlreadyPresent {
                    gc: self.control.to_string(),
                    game: self.game.nanoid.to_owned(),
                    turn: format!("{}", self.game.turn),
                })?
            }
        }
        Ok(())
    }

    fn ensure_previous_gc_present(&self) -> Result<()> {
        let opposite_color = self.control.color().opposite_color();
        let should_be_gc = match self.control {
            GameControl::TakebackAccept(_) => GameControl::TakebackRequest(opposite_color),
            GameControl::TakebackReject(_) => GameControl::TakebackRequest(opposite_color),
            GameControl::DrawReject(_) => GameControl::DrawOffer(opposite_color),
            GameControl::DrawAccept(_) => GameControl::DrawOffer(opposite_color),
            _ => unreachable!(),
        };
        if let Some(last_gc) = self.game.last_game_control() {
            if last_gc == should_be_gc {
                return Ok(());
            }
        }
        Err(GameError::InvalidGc {
            gc: self.control.to_string(),
            game: self.game.nanoid.to_owned(),
            turn: format!("{}", self.game.turn),
        })?
    }

    async fn handle_takeback_request(&self) -> Result<Game> {
        let game = self
            .game
            .write_game_control(&self.control, &self.pool)
            .await?;
        Ok(game)
    }

    async fn handle_takeback_accept(&self) -> Result<Game> {
        self.ensure_previous_gc_present()?;
        let game = self.game.accept_takeback(&self.control, &self.pool).await?;
        Ok(game)
    }

    async fn handle_resign(&self) -> Result<Game> {
        let game = self.game.resign(&self.control, &self.pool).await?;
        Ok(game)
    }

    async fn handle_abort(&self) -> Result<()> {
        Ok(self.game.delete(&self.pool).await?)
    }

    async fn handle_draw_reject(&self) -> Result<Game> {
        self.ensure_previous_gc_present()?;
        let game = self
            .game
            .write_game_control(&self.control, &self.pool)
            .await?;
        Ok(game)
    }

    async fn handle_draw_offer(&self) -> Result<Game> {
        let game = self
            .game
            .write_game_control(&self.control, &self.pool)
            .await?;
        Ok(game)
    }

    async fn handle_draw_accept(&self) -> Result<Game> {
        self.ensure_previous_gc_present()?;
        let game = self.game.accept_draw(&self.control, &self.pool).await?;
        Ok(game)
    }

    async fn handle_takeback_reject(&self) -> Result<Game> {
        self.ensure_previous_gc_present()?;
        let game = self
            .game
            .write_game_control(&self.control, &self.pool)
            .await?;
        Ok(game)
    }

    async fn match_control(&self) -> Result<Game> {
        Ok(match self.control {
            GameControl::Abort(_) => {
                let game = self.game.clone();
                self.handle_abort().await?;
                game
            }
            GameControl::Resign(_) => self.handle_resign().await?,
            GameControl::DrawOffer(_) => self.handle_draw_offer().await?,
            GameControl::DrawAccept(_) => self.handle_draw_accept().await?,
            GameControl::DrawReject(_) => self.handle_draw_reject().await?,
            GameControl::TakebackRequest(_) => self.handle_takeback_request().await?,
            GameControl::TakebackAccept(_) => self.handle_takeback_accept().await?,
            GameControl::TakebackReject(_) => self.handle_takeback_reject().await?,
        })
    }

    fn ensure_gc_allowed_for_turn(&self) -> Result<()> {
        if let Some(color) = self.game.user_color(self.user_id) {
            if !self.control.allowed_on_turn(self.game.turn, color) {
                Err(GameError::InvalidGc {
                    gc: self.control.to_string(),
                    game: self.game.nanoid.to_owned(),
                    turn: format!("{}", self.game.turn),
                })?
            }
            return Ok(());
        }
        Err(GameError::InvalidGc {
            gc: self.control.to_string(),
            game: self.game.nanoid.to_owned(),
            turn: format!("{}", self.game.turn),
        })?
    }

    fn ensure_gc_color(&self) -> Result<()> {
        if let Some(color) = self.game.user_color(self.user_id) {
            if color == self.control.color() {
                return Ok(());
            }
        }
        Err(GameError::InvalidGc {
            gc: self.control.to_string(),
            game: self.game.nanoid.to_owned(),
            turn: format!("{}", self.game.turn),
        })?
    }
}
