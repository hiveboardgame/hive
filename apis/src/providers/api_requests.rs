use crate::common::challenge_action::ChallengeAction;
use crate::common::{client_message::ClientRequest, game_action::GameAction};
use crate::providers::web_socket::WebsocketContext;
use hive_lib::{game_control::GameControl, turn::Turn};
use leptos::*;

use super::games::GamesSignal;

#[derive(Clone)]
pub struct ApiRequests {
    websocket: WebsocketContext,
}

impl Default for ApiRequests {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiRequests {
    pub fn new() -> Self {
        let websocket = expect_context::<WebsocketContext>();
        Self { websocket }
    }

    pub fn turn(&self, game_id: String, turn: Turn) {
        let msg = ClientRequest::Game {
            id: game_id.to_owned(),
            action: GameAction::Move(turn),
        };
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
        let mut games = expect_context::<GamesSignal>();
        games.games_remove(&game_id);
    }

    pub fn game_control(&self, game_id: String, gc: GameControl) {
        let msg = ClientRequest::Game {
            id: game_id,
            action: GameAction::Control(gc),
        };
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn game_check_time(&self, game_id: &str) {
        let msg = ClientRequest::GameTimeout(game_id.to_owned());
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn join(&self, game_id: String) {
        let msg = ClientRequest::Game {
            id: game_id,
            action: GameAction::Join,
        };
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn challenge(&self, challenge_action: ChallengeAction) {
        let msg = ClientRequest::Challenge(challenge_action);
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn challenge_cancel(&self, nanoid: String) {
        let msg = ClientRequest::Challenge(ChallengeAction::Delete(nanoid));
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn challenge_accept(&self, nanoid: String) {
        let msg = ClientRequest::Challenge(ChallengeAction::Accept(nanoid));
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn challenge_get(&self, nanoid: String) {
        let msg = ClientRequest::Challenge(ChallengeAction::Get(nanoid));
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }
}
