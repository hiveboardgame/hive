use crate::common::challenge_action::{ChallengeAction, ChallengeVisibility};
use crate::common::{client_message::ClientRequest, game_action::GameAction};
use crate::pages::challenge_create::ChallengeParams;
use crate::providers::web_socket::WebsocketContext;
use hive_lib::color::ColorChoice;
use hive_lib::game_type::GameType;
use hive_lib::{game_control::GameControl, turn::Turn};
use leptos::*;

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
            id: game_id,
            action: GameAction::Move(turn),
        };
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

    pub fn join(&self, game_id: String) {
        let msg = ClientRequest::Game {
            id: game_id,
            action: GameAction::Join,
        };
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }

    pub fn challenge_new(&self) {
        let msg = ClientRequest::Challenge(ChallengeAction::Create {
            rated: true,
            game_type: GameType::MLP,
            visibility: ChallengeVisibility::Public,
            opponent: None,
            color_choice: ColorChoice::Random,
        });
        self.websocket
            .send(&serde_json::to_string(&msg).expect("Serde_json::to_string failed"));
    }
    pub fn challenge_new_with_params(&self, params: ChallengeParams) {
        let msg = ClientRequest::Challenge(ChallengeAction::Create {
            rated: params.rated.get_untracked(),
            game_type: params.game_type.get_untracked(),
            visibility: params.visibility.get_untracked(),
            opponent: params.opponent.get_untracked(),
            color_choice: params.color_choice.get_untracked(),
        });
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
