use super::{
    control_handler::GameControlHandler,
    join_handler::JoinHandler,
    start::StartHandler,
    timeout_handler::TimeoutHandler,
    turn_handler::{TurnHandler, TurnInput},
};
use crate::{
    common::GameAction,
    websocket::{
        messages::{HandlerOutput, SocketTx},
        server_handlers::tournaments::berserk::BerserkHandler,
        WebsocketData,
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use shared_types::GameId;
use std::sync::Arc;
use uuid::Uuid;
pub struct GameActionHandler {
    game_action: GameAction,
    game: Game,
    pool: DbPool,
    user_id: Uuid,
    received_from: SocketTx,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
    username: String,
}

impl GameActionHandler {
    pub async fn new(
        game_id: &GameId,
        game_action: GameAction,
        received_from: SocketTx,
        user_details: (&str, Uuid),
        data: Arc<WebsocketData>,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Result<Self> {
        let (username, user_id) = user_details;
        let mut connection = get_conn(pool).await?;

        let game = Game::find_by_game_id(game_id, &mut connection).await?;

        Ok(Self {
            pool: pool.clone(),
            data,
            hub,
            game,
            received_from,
            username: username.to_owned(),
            game_action,
            user_id,
        })
    }

    async fn turn(&self, input: TurnInput, expected_turn: Option<usize>) -> Result<HandlerOutput> {
        TurnHandler::new(
            input,
            expected_turn,
            &self.game,
            &self.username,
            self.user_id,
            self.data.clone(),
            &self.pool,
        )
        .handle()
        .await
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let _start = if can_start_game(&self.game_action) {
            self.data.realtime_gate.admit_start(&self.game)?
        } else {
            None
        };
        let output = match self.game_action.clone() {
            GameAction::Berserk => {
                BerserkHandler::new(self.game.id, self.user_id, self.data.clone(), &self.pool)
                    .handle()
                    .await?
            }
            GameAction::CheckTime => {
                TimeoutHandler::new(
                    &self.game,
                    self.received_from.clone(),
                    self.data.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Turn(turn) => self.turn(TurnInput::Structured(turn), None).await?,
            GameAction::Play(notation) => self.turn(TurnInput::Notation(notation), None).await?,
            GameAction::PlayAt { turn, notation } => {
                self.turn(TurnInput::Notation(notation), Some(turn)).await?
            }
            GameAction::Control(control) => {
                GameControlHandler::new(
                    &control,
                    &self.game,
                    &self.username,
                    self.user_id,
                    self.data.clone(),
                    self.hub.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Join => {
                JoinHandler::new(
                    &self.game,
                    &self.username,
                    self.user_id,
                    self.received_from.clone(),
                    self.data.clone(),
                    self.hub.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Start => {
                StartHandler::new(
                    &self.game,
                    self.user_id,
                    self.username.clone(),
                    self.data.clone(),
                    &self.pool,
                )
                .handle()
                .await?
            }
            GameAction::Unwatch => {
                unreachable!("Unwatch is intercepted in handle_binary before GameActionHandler")
            }
        };
        Ok(output)
    }
}

fn can_start_game(action: &GameAction) -> bool {
    match action {
        GameAction::Turn(_)
        | GameAction::Play(_)
        | GameAction::PlayAt { .. }
        | GameAction::Start => true,
        GameAction::Berserk
        | GameAction::CheckTime
        | GameAction::Control(_)
        | GameAction::Join
        | GameAction::Unwatch => false,
    }
}

#[cfg(test)]
mod tests {
    use super::can_start_game;
    use crate::common::GameAction;
    use hive_lib::{Color, GameControl, Turn};

    #[test]
    fn every_way_to_move_or_start_needs_the_realtime_gate() {
        for action in [
            GameAction::Turn(Turn::Shutout),
            GameAction::Play("wG1".to_string()),
            GameAction::PlayAt {
                turn: 0,
                notation: "wG1".to_string(),
            },
            GameAction::Start,
        ] {
            assert!(can_start_game(&action), "{action:?} bypasses the gate");
        }
        for action in [
            GameAction::Berserk,
            GameAction::CheckTime,
            GameAction::Control(GameControl::Abort(Color::White)),
            GameAction::Join,
            GameAction::Unwatch,
        ] {
            assert!(!can_start_game(&action), "{action:?} is gated");
        }
    }
}
