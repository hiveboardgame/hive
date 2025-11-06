use crate::common::{ChallengeAction, ClientRequest, GameAction, ScheduleAction, TournamentAction};
use crate::providers::{challenges::ChallengeStateSignal, AuthContext};
use crate::responses::create_challenge_handler;
use futures::{channel::mpsc::Sender, SinkExt};
use hive_lib::{GameControl, Turn};
use leptos::prelude::{With, WithUntracked};
use leptos::{
    logging,
    prelude::{expect_context, GetValue, ReadSignal, RwSignal, Set, SetValue, StoredValue},
};
use server_fn::ServerFnError;
use shared_types::{ChallengeId, GameId, TournamentGameResult, TournamentId};
pub type ClientResult = Result<ClientRequest, ServerFnError>;

#[derive(Clone, Copy)]
pub struct ClientApi {
    ws_restart: RwSignal<()>,
    game_join: RwSignal<()>,
    //client api holds the client mpsc sender
    sender: StoredValue<Option<Sender<ClientResult>>>,
    ws_ready:  RwSignal<bool>
}
impl Default for ClientApi {
    fn default() -> Self {
        Self {
            ws_restart: RwSignal::new(()),
            game_join: RwSignal::new(()),
            sender: StoredValue::new(None),
            ws_ready: RwSignal::new(false)
        }
    }
}
impl ClientApi {
    pub fn set_sender(&self, sender: Option<Sender<ClientResult>>) {
        self.sender.set_value(sender);
    }
    pub fn restart_ws(&self) {
        self.set_sender(None);
        self.ws_restart.set(());
    }
    pub fn signal_restart_ws(&self) -> ReadSignal<()> {
        self.ws_restart.read_only()
    }
    pub fn game_join(&self) {
        self.game_join.set(());
    }
    pub fn set_ws_ready(&self) {
        self.ws_ready.set(true);
    }
    pub fn set_ws_pending(&self) {
        self.ws_ready.set(false);
    }
    pub fn signal_ws_ready(&self) ->ReadSignal<bool> {
        self.ws_ready.read_only()
    }
    pub fn signal_game_join(&self) -> ReadSignal<()> {
        self.game_join.read_only()
    }
    fn send(&self, client_request: ClientRequest) {
        let sender = self.sender.get_value();
        let ret = sender.expect("Dont have a sender").try_send(Ok(client_request.clone()));
        if ret.is_err() {
            logging::log!("Msg: {client_request:?} Error: {ret:?}");
        }
    }
    pub fn link_discord(&self) {
        self.send(ClientRequest::LinkDiscord);
    }
    pub fn join_game(&self, id: GameId) {
        let req = ClientRequest::Game {
            game_id: id,
            action: GameAction::Join,
        };
        self.send(req);
    }
    pub fn subscribe_tournament(&self, id: TournamentId) {
        let req = ClientRequest::Tournament(TournamentAction::Subscribe(id));
        self.send(req);
    }
    pub fn turn(&self, game_id: GameId, turn: Turn) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Turn(turn),
        };
        self.send(msg);
    }
    pub async fn pong(&self, nonce: u64) {
        let sender = self.sender.get_value();
        let ret = sender.expect("Dont have a sender").send(Ok( ClientRequest::Pong(nonce))).await;
        if ret.is_err() {
            logging::log!("Pong Error: {ret:?}");
        }
    }
    pub fn game_control(&self, game_id: GameId, gc: GameControl) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Control(gc),
        };
        self.send(msg);
    }
    pub fn game_check_time(&self, game_id: GameId) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::CheckTime,
        };
        self.send(msg);
    }
    pub fn tournament_game_start(&self, game_id: GameId) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Start,
        };
        self.send(msg);
    }
    pub fn challenge(&self, challenge_action: ChallengeAction) {
        let challenge_action = match challenge_action {
            ChallengeAction::Create(details) => {
                let auth_context = expect_context::<AuthContext>();
                let challenges = expect_context::<ChallengeStateSignal>();
                auth_context.user.with(|a| {
                    a.as_ref().and_then(|account| {
                        let challenge_list = challenges
                            .signal
                            .with_untracked(|c| c.challenges.values().cloned().collect::<Vec<_>>());
                        create_challenge_handler(
                            account.user.username.clone(),
                            details,
                            challenge_list,
                        )
                    })
                })
            }
            other => Some(other),
        };
        if let Some(challenge_action) = challenge_action {
            let msg = ClientRequest::Challenge(challenge_action);
            self.send(msg);
        }
    }
    pub fn challenge_cancel(&self, challenger_id: ChallengeId) {
        self.challenge(ChallengeAction::Delete(challenger_id));
    }
    pub fn challenge_accept(&self, challenger_id: ChallengeId) {
        self.challenge(ChallengeAction::Accept(challenger_id));
    }
    pub fn challenge_get(&self, challenger_id: ChallengeId) {
        self.challenge(ChallengeAction::Get(challenger_id));
    }
    pub fn schedule_action(&self, action: ScheduleAction) {
        self.send(ClientRequest::Schedule(action));
    }
    pub fn tournament(&self, action: TournamentAction) {
        self.send(ClientRequest::Tournament(action));
    }
    pub fn tournament_abandon(&self, tournament_id: TournamentId) {
        self.tournament(TournamentAction::Abandon(tournament_id));
    }
    pub fn tournament_adjudicate_game_result(
        &self,
        game_id: GameId,
        new_result: TournamentGameResult,
    ) {
        self.tournament(TournamentAction::AdjudicateResult(game_id, new_result));
    }
}
