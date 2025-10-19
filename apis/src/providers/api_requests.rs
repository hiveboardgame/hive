use super::challenges::ChallengeStateSignal;
use super::websocket;
use crate::common::{ClientRequest, GameAction, ScheduleAction, TournamentAction};
use crate::providers::websocket::WebsocketContext;
use hive_lib::GameControl;
use leptos::prelude::*;
use shared_types::{ChatMessageContainer, GameId, TournamentGameResult, TournamentId};

#[derive(Clone)]
pub struct ApiRequests {
    websocket: WebsocketContext,
    pub challenges: ChallengeStateSignal,
}

#[derive(Clone)]
pub struct ApiRequestsProvider(pub Signal<ApiRequests>);

impl ApiRequests {
    pub fn new(
        websocket: websocket::WebsocketContext,
        challenges: ChallengeStateSignal,
    ) -> Self {
        Self {
            websocket,
            challenges,
        }
    }


    pub fn link_discord(&self) {
        let msg = ClientRequest::LinkDiscord;
        self.websocket.send(&msg);
    }

    pub fn game_control(&self, game_id: GameId, gc: GameControl) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Control(gc),
        };
        self.websocket.send(&msg);
    }

    pub fn tournament_game_start(&self, game_id: GameId) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Start,
        };
        self.websocket.send(&msg);
    }

    pub fn tournament_abandon(&self, tournament_id: TournamentId) {
        let msg = ClientRequest::Tournament(TournamentAction::Abandon(tournament_id));
        self.websocket.send(&msg);
    }

    pub fn tournament_adjudicate_game_result(
        &self,
        game_id: GameId,
        new_result: TournamentGameResult,
    ) {
        let msg =
            ClientRequest::Tournament(TournamentAction::AdjudicateResult(game_id, new_result));
        self.websocket.send(&msg);
    }

    pub fn chat(&self, message: &ChatMessageContainer) {
        let msg = ClientRequest::Chat(message.to_owned());
        self.websocket.send(&msg);
    }

    pub fn tournament(&self, action: TournamentAction) {
        let msg = ClientRequest::Tournament(action.to_owned());
        self.websocket.send(&msg);
    }

    pub fn game_check_time(&self, game_id: &GameId) {
        let msg = ClientRequest::Game {
            game_id: game_id.clone(),
            action: GameAction::CheckTime,
        };
        self.websocket.send(&msg);
    }


    pub fn schedule_action(&self, action: ScheduleAction) {
        let msg = ClientRequest::Schedule(action);
        self.websocket.send(&msg);
    }
}

pub fn provide_api_requests() {
    let ws = expect_context::<WebsocketContext>();
    let challenges = expect_context::<ChallengeStateSignal>();
    let api_requests = ApiRequests::new(ws, challenges);
    provide_context(ApiRequestsProvider(Signal::derive(move || {
        api_requests.clone()
    })));
}
