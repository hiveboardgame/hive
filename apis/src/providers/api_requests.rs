use crate::common::ChallengeAction;
use crate::common::{ClientRequest, GameAction};
use crate::providers::websocket::WebsocketContext;
use chrono::Utc;
use hive_lib::{GameControl, Turn};
use leptos::*;
use shared_types::ChatMessageContainer;

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
            action: GameAction::Turn(turn),
        };
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
        let mut games = expect_context::<GamesSignal>();
        // TODO: fix this so that it just removes from next_games games.remove_from_next_games(&game_id);
        games.own_games_remove(&game_id);
    }

    pub fn ping(&self) {
        let msg = ClientRequest::Ping(Utc::now());
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn game_control(&self, game_id: String, gc: GameControl) {
        let msg = ClientRequest::Game {
            id: game_id,
            action: GameAction::Control(gc),
        };
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn chat(&self, message: &ChatMessageContainer) {
        let msg = ClientRequest::Chat(message.to_owned());
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn game_check_time(&self, game_id: &str) {
        let msg = ClientRequest::Game {
            id: game_id.to_owned(),
            action: GameAction::CheckTime,
        };
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
