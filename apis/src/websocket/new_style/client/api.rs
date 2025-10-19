use crate::common::{ChallengeAction, ClientRequest, GameAction, ScheduleAction};
use crate::providers::{AuthContext, challenges::ChallengeStateSignal};
use crate::responses::create_challenge_handler;
use futures::{
    channel::mpsc::{self, Sender},
    SinkExt,
};
use hive_lib::{GameControl, Turn};
use leptos::prelude::{With, WithUntracked};
use leptos::{
    logging,
    prelude::{expect_context, GetValue, ReadSignal, RwSignal, Set, SetValue, StoredValue},
};
use server_fn::ServerFnError;
use shared_types::{ChallengeId, GameId};
pub type ClientResult = Result<ClientRequest, ServerFnError>;

#[derive(Clone, Copy)]
pub struct ClientApi {
    ws_restart: RwSignal<()>,
    game_join: RwSignal<()>,
    //client api holds the client mpsc sender
    //and the latest message received from the server
    sender: StoredValue<Sender<ClientResult>>,
}
impl Default for ClientApi {
    fn default() -> Self {
        let (tx, _rx) = mpsc::channel(1);
        Self {
            ws_restart: RwSignal::new(()),
            game_join: RwSignal::new(()),
            sender: StoredValue::new(tx),
        }
    }
}
impl ClientApi {
    pub fn set_sender(&self, sender: Sender<ClientResult>) {
        self.sender.set_value(sender);
    }
    pub fn restart_ws(&self) {
        self.ws_restart.set(());
    }
    pub fn signal_restart_ws(&self) -> ReadSignal<()> {
        self.ws_restart.read_only()
    }
    pub fn game_join(&self) {
        self.game_join.set(());
    }
    pub fn signal_game_join(&self) -> ReadSignal<()> {
        self.game_join.read_only()
    }
    async fn send(&self, client_request: ClientRequest) {
        let mut sender = self.sender.get_value();
        let ret = sender.send(Ok(client_request.clone())).await;
        if ret.is_err() {
            logging::log!("Msg: {client_request:?} Error: {ret:?}");
        }
    }
    pub async fn join_game(&self, id: GameId) {
        let req = ClientRequest::Game {
            game_id: id,
            action: GameAction::Join,
        };
        self.send(req).await;
    }
    pub async fn turn(&self, game_id: GameId, turn: Turn) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Turn(turn),
        };
        self.send(msg).await;
    }
    pub async fn pong(&self, nonce: u64) {
        let msg = ClientRequest::Pong(nonce);
        self.send(msg).await;
    }
    pub async fn game_control(&self, game_id: GameId, gc: GameControl) {
        let msg = ClientRequest::Game {
            game_id,
            action: GameAction::Control(gc),
        };
        self.send(msg).await;
    }
    pub async fn challenge(&self, challenge_action: ChallengeAction) {
        let challenge_action = match challenge_action {
            ChallengeAction::Create(details) => {
                let auth_context = expect_context::<AuthContext>();
                let challenges = expect_context::<ChallengeStateSignal>();
                auth_context.user.with(|a| {
                    a.as_ref().and_then(|account| {
                        let challenge_list = challenges.signal.with_untracked(|c| {
                            c.challenges.values().cloned().collect::<Vec<_>>()
                        });
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
            self.send(msg).await;
        }
    }
    pub async fn challenge_cancel(&self, challenger_id: ChallengeId) {
        self.challenge(ChallengeAction::Delete(challenger_id)).await;
    }
    pub async fn challenge_accept(&self, challenger_id: ChallengeId) {
        self.challenge(ChallengeAction::Accept(challenger_id)).await;
    }
    pub async fn challenge_get(&self, challenger_id: ChallengeId) {
        self.challenge(ChallengeAction::Get(challenger_id)).await;
    }
    pub async fn schedule_action(&self, action: ScheduleAction) {
        self.send(ClientRequest::Schedule(action)).await;
    }
}
