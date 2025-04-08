use super::challenges::ChallengeStateSignal;
use super::games::GamesSignal;
use super::{auth_context, websocket};
use crate::common::{ChallengeAction, ScheduleAction, TournamentAction};
use crate::common::{ClientRequest, GameAction};
use crate::providers::websocket::WebsocketContext;
use crate::responses::{create_challenge_handler, AccountResponse};
use hive_lib::{GameControl, Turn};
use leptos::prelude::*;
use shared_types::{ChallengeId, ChatMessageContainer, GameId, TournamentGameResult, TournamentId};

#[derive(Clone)]
pub struct ApiRequests {
    websocket: WebsocketContext,
    user: Signal<Option<AccountResponse>>,
    pub challenges: ChallengeStateSignal,
}

#[derive(Clone)]
pub struct ApiRequestsProvider(pub Signal<ApiRequests>);

impl ApiRequests {
    pub fn new(
        websocket: websocket::WebsocketContext,
        user: Signal<Option<AccountResponse>>,
        challenges: ChallengeStateSignal,
    ) -> Self {
        Self {
            websocket,
            user,
            challenges,
        }
    }

    pub fn turn(&self, game_id: GameId, turn: Turn) {
        let msg = ClientRequest::Game {
            game_id: game_id.clone(),
            action: GameAction::Turn(turn),
        };
        self.websocket.send(&msg);
        let mut games = expect_context::<GamesSignal>();
        // TODO: fix this so that it just removes from next_games games.remove_from_next_games(&game_id);
        games.own_games_remove(&game_id);
    }

    pub fn pong(&self, nonce: u64) {
        let msg = ClientRequest::Pong(nonce);
        self.websocket.send(&msg);
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

    pub fn challenge(&self, challenge_action: ChallengeAction) {
        let challenge_action = match challenge_action {
            ChallengeAction::Create(details) => {
                let account = self.user.get();
                if let Some(account) = account {
                    let challenges = self.challenges.signal.get_untracked();
                    let challenges = challenges.challenges.into_values().collect();
                    create_challenge_handler(account.user.username, details, challenges)
                } else {
                    None
                }
            }
            other => Some(other),
        };
        if let Some(challenge_action) = challenge_action {
            let msg = ClientRequest::Challenge(challenge_action);
            self.websocket.send(&msg);
        }
    }

    pub fn challenge_cancel(&self, challenger_id: ChallengeId) {
        let msg = ClientRequest::Challenge(ChallengeAction::Delete(challenger_id));
        self.websocket.send(&msg);
    }

    pub fn challenge_accept(&self, challenger_id: ChallengeId) {
        let msg = ClientRequest::Challenge(ChallengeAction::Accept(challenger_id));
        self.websocket.send(&msg);
    }

    pub fn challenge_get(&self, challenger_id: ChallengeId) {
        let msg = ClientRequest::Challenge(ChallengeAction::Get(challenger_id));
        self.websocket.send(&msg);
    }

    pub fn join(&self, game_id: GameId) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Join,
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
    let auth_context = expect_context::<auth_context::AuthContext>();
    let challenges = expect_context::<ChallengeStateSignal>();
    let api_requests = ApiRequests::new(ws, auth_context.user, challenges);
    provide_context(ApiRequestsProvider(Signal::derive(move || {
        api_requests.clone()
    })));
}
