use crate::{
    bot::{self, Turn},
    config::BotConfig,
    protocol::{self, Challenge, ChallengeAction, ClientRequest, Game, Incoming, User},
    token::{self, TokenError},
    turn_tracker::{TurnTracker, TurnTracking},
};
use futures_util::{SinkExt, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{mpsc, Semaphore},
    time::{interval_at, sleep, timeout, Instant, MissedTickBehavior},
};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{self, protocol::WebSocketConfig, Message},
    MaybeTlsStream,
    WebSocketStream,
};
use tracing::{debug, error, info, warn};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const AUTH_TIMEOUT: Duration = Duration::from_secs(30);
// Pushes cover every move; this only retries turns whose engine run failed.
const RECONCILE_INTERVAL: Duration = Duration::from_secs(30);
const RECONNECT_DELAY_MIN: Duration = Duration::from_secs(1);
const RECONNECT_DELAY_MAX: Duration = Duration::from_secs(10);

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error(transparent)]
    Token(#[from] TokenError),
    #[error("websocket: {0}")]
    WebSocket(#[from] tungstenite::Error),
    #[error("could not encode request: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("authentication refused: {0}")]
    AuthRefused(String),
    #[error("no answer to Auth within {AUTH_TIMEOUT:?}")]
    AuthTimeout,
    #[error("connection closed during authentication")]
    ClosedDuringAuth,
}

pub struct Shared {
    pub http: reqwest::Client,
    pub base_url: String,
    pub websocket_url: String,
    pub engine_slots: Arc<Semaphore>,
}

struct Session {
    bot: Arc<BotConfig>,
    shared: Arc<Shared>,
    me: User,
    outbox: mpsc::UnboundedSender<ClientRequest>,
    turn_tracker: TurnTracker,
}

pub async fn run(bot: BotConfig, shared: Arc<Shared>) {
    let bot = Arc::new(bot);
    // Outlives each connection, so a move whose engine finishes during a reconnect
    // goes out on the next socket.
    let (outbox, mut queued) = mpsc::unbounded_channel();
    let turn_tracker = TurnTracker::new();
    let mut reconnect_delay = RECONNECT_DELAY_MIN;
    loop {
        match connect(&bot, &shared).await {
            Ok((socket, me)) => {
                info!(bot = %bot.name, username = %me.username, "connected");
                reconnect_delay = RECONNECT_DELAY_MIN;
                // A move written to a dying socket may never have arrived; if so the game is
                // still pending and must be played again.
                turn_tracker.forget_processed().await;
                let session = Session {
                    bot: bot.clone(),
                    shared: shared.clone(),
                    me,
                    outbox: outbox.clone(),
                    turn_tracker: turn_tracker.clone(),
                };
                match session.serve(socket, &mut queued).await {
                    Ok(()) => info!(bot = %bot.name, "connection closed"),
                    Err(e) => warn!(bot = %bot.name, error = %e, "connection lost"),
                }
            }
            Err(e) => error!(bot = %bot.name, error = %e, "connection failed"),
        }
        sleep(reconnect_delay).await;
        reconnect_delay = (reconnect_delay * 2).min(RECONNECT_DELAY_MAX);
    }
}

async fn connect(bot: &BotConfig, shared: &Shared) -> Result<(Socket, User), SessionError> {
    info!(bot = %bot.name, email = %bot.email, "fetching token");
    let token = token::fetch(&shared.http, &shared.base_url, &bot.email, &bot.password).await?;
    // A game list is a single frame and runs to megabytes for a bot with many games.
    let config = WebSocketConfig::default()
        .max_message_size(None)
        .max_frame_size(None);
    let (mut socket, _) =
        connect_async_with_config(shared.websocket_url.as_str(), Some(config), true).await?;
    send(&mut socket, &ClientRequest::Auth(token)).await?;
    let me = timeout(AUTH_TIMEOUT, await_authentication(&mut socket))
        .await
        .map_err(|_| SessionError::AuthTimeout)??;
    Ok((socket, me))
}

async fn await_authentication(socket: &mut Socket) -> Result<User, SessionError> {
    while let Some(frame) = socket.next().await {
        let Message::Text(text) = frame? else {
            continue;
        };
        match protocol::parse(&text) {
            Ok(Incoming::Authenticated(me)) => return Ok(me),
            Ok(Incoming::Ping { nonce }) => send(socket, &ClientRequest::Pong(nonce)).await?,
            Ok(Incoming::Error(reason)) => return Err(SessionError::AuthRefused(reason)),
            _ => {}
        }
    }
    Err(SessionError::ClosedDuringAuth)
}

async fn send(socket: &mut Socket, request: &ClientRequest) -> Result<(), SessionError> {
    socket
        .send(Message::text(serde_json::to_string(request)?))
        .await?;
    Ok(())
}

impl Session {
    async fn serve(
        &self,
        mut socket: Socket,
        queued: &mut mpsc::UnboundedReceiver<ClientRequest>,
    ) -> Result<(), SessionError> {
        send(&mut socket, &ClientRequest::GetPendingGames).await?;
        send(&mut socket, &ClientRequest::Resync).await?;
        let mut reconcile = interval_at(Instant::now() + RECONCILE_INTERVAL, RECONCILE_INTERVAL);
        reconcile.set_missed_tick_behavior(MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                frame = socket.next() => match frame {
                    None => return Ok(()),
                    Some(frame) => {
                        if let Message::Text(text) = frame? {
                            self.handle(&mut socket, &text).await?;
                        }
                    }
                },
                Some(request) = queued.recv() => send(&mut socket, &request).await?,
                _ = reconcile.tick() => {
                    self.turn_tracker.cleanup().await;
                    send(&mut socket, &ClientRequest::GetPendingGames).await?;
                }
            }
        }
    }

    async fn handle(&self, socket: &mut Socket, text: &str) -> Result<(), SessionError> {
        let bot = &self.bot.name;
        let incoming = match protocol::parse(text) {
            Ok(incoming) => incoming,
            Err(e) => {
                warn!(bot = %bot, error = %e, "unreadable server message");
                return Ok(());
            }
        };
        match incoming {
            Incoming::Ping { nonce } => send(socket, &ClientRequest::Pong(nonce)).await?,
            Incoming::Games(games) => self.take_turns(games).await,
            Incoming::Challenges(challenges) => self.accept_challenges(socket, challenges).await?,
            Incoming::LobbySnapshot(snapshot) => {
                self.take_turns(snapshot.urgent_games).await;
                self.accept_challenges(socket, snapshot.challenges).await?;
            }
            Incoming::ResyncRetry { retry_after_ms } => {
                self.resync_later(Duration::from_millis(retry_after_ms))
            }
            Incoming::Error(reason) => warn!(bot = %bot, reason = %reason, "server error"),
            Incoming::Refused(refusal) => warn!(bot = %bot, refusal = %refusal, "request refused"),
            Incoming::Authenticated(_) => {}
            Incoming::Other(tag) => debug!(bot = %bot, tag = %tag, "ignored server message"),
        }
        Ok(())
    }

    async fn take_turns(&self, games: Vec<Game>) {
        for game in games {
            if !game.is_turn_of(&self.me.uid) {
                continue;
            }
            let position_key = game.position_key();
            if self.turn_tracker.tracked(position_key).await {
                continue;
            }
            self.turn_tracker.processing(position_key).await;
            debug!(bot = %self.bot.name, game_id = %game.game_id, "our turn");
            tokio::spawn(bot::play_turn(
                Turn {
                    game,
                    position_key,
                    bot: self.bot.clone(),
                    outbox: self.outbox.clone(),
                },
                self.shared.engine_slots.clone(),
                self.turn_tracker.clone(),
            ));
        }
    }

    async fn accept_challenges(
        &self,
        socket: &mut Socket,
        challenges: Vec<Challenge>,
    ) -> Result<(), SessionError> {
        for challenge in challenges {
            let addressed_to_us = challenge
                .opponent
                .as_ref()
                .is_some_and(|opponent| opponent.uid == self.me.uid);
            if !addressed_to_us {
                continue;
            }
            info!(bot = %self.bot.name, challenge_id = %challenge.challenge_id, "accepting challenge");
            let accept = ClientRequest::Challenge(ChallengeAction::Accept(challenge.challenge_id));
            send(socket, &accept).await?;
        }
        Ok(())
    }

    fn resync_later(&self, delay: Duration) {
        let outbox = self.outbox.clone();
        tokio::spawn(async move {
            sleep(delay).await;
            let _ = outbox.send(ClientRequest::Resync);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::fs;
    use tempfile::TempDir;
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;
    use wiremock::{
        matchers::{method, path},
        Mock,
        MockServer,
        ResponseTemplate,
    };

    const ME: &str = "00000000-0000-0000-0000-000000000001";
    const THEM: &str = "00000000-0000-0000-0000-000000000002";
    const PATIENCE: Duration = Duration::from_secs(10);

    struct Hivegame {
        _http: MockServer,
        listener: TcpListener,
        _engine_dir: TempDir,
        session: tokio::task::JoinHandle<()>,
    }

    struct Connection {
        socket: WebSocketStream<TcpStream>,
    }

    impl Hivegame {
        async fn start() -> Self {
            let http = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path("/api/v1/auth/token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "success": true,
                    "data": {"token": "tok"}
                })))
                .mount(&http)
                .await;
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let engine_dir = TempDir::new().unwrap();
            let engine = engine_dir.path().join("engine.sh");
            fs::write(
                &engine,
                "cat > /dev/null\nprintf 'id fake\\nok\\nBase+MLP;InProgress;Black[1];wS1\\nok\\nbG1 -wS1\\nok\\n'\n",
            )
            .unwrap();
            let bot = BotConfig {
                name: "test-bot".to_string(),
                ai_command: format!("sh {}", engine.display()),
                bestmove_command_args: "depth 1".to_string(),
                email: "bot@example.com".to_string(),
                password: "hivegame".to_string(),
            };
            let shared = Arc::new(Shared {
                http: reqwest::Client::new(),
                base_url: http.uri(),
                websocket_url: format!("ws://{}/ws/?format=json", listener.local_addr().unwrap()),
                engine_slots: Arc::new(Semaphore::new(1)),
            });
            let session = tokio::spawn(run(bot, shared));
            Self {
                _http: http,
                listener,
                _engine_dir: engine_dir,
                session,
            }
        }

        async fn accept(&self) -> Connection {
            let (tcp, _) = timeout(PATIENCE, self.listener.accept())
                .await
                .expect("hydra never connected")
                .unwrap();
            Connection {
                socket: accept_async(tcp).await.unwrap(),
            }
        }

        async fn authenticated(&self) -> Connection {
            let mut connection = self.accept().await;
            assert_eq!(connection.next_frame().await, json!({"Auth": "tok"}));
            connection
                .push(json!({"Ok": {"Authenticated": user(ME)}}))
                .await;
            assert_eq!(connection.next_frame().await, json!("GetPendingGames"));
            assert_eq!(connection.next_frame().await, json!("Resync"));
            connection
        }
    }

    impl Drop for Hivegame {
        fn drop(&mut self) {
            self.session.abort();
        }
    }

    impl Connection {
        async fn push(&mut self, message: Value) {
            self.socket
                .send(Message::text(message.to_string()))
                .await
                .unwrap();
        }

        async fn next_frame(&mut self) -> Value {
            loop {
                let frame = timeout(PATIENCE, self.socket.next())
                    .await
                    .expect("hydra went quiet")
                    .expect("hydra hung up")
                    .unwrap();
                if let Message::Text(text) = frame {
                    return serde_json::from_str(&text).unwrap();
                }
            }
        }
    }

    fn user(uid: &str) -> Value {
        json!({"username": "someone", "uid": uid})
    }

    fn game(game_id: &str, current_player_id: &str) -> Value {
        json!({
            "game_id": game_id, "current_player_id": current_player_id, "finished": false,
            "game_type": "MLP", "history": [["wS1", ""]]
        })
    }

    #[tokio::test]
    async fn answers_pings_while_authenticating() {
        let hivegame = Hivegame::start().await;
        let mut connection = hivegame.accept().await;

        assert_eq!(connection.next_frame().await, json!({"Auth": "tok"}));
        connection
            .push(
                json!({"Ok": {"UserStatus": {"status": "Online", "user": null, "username": "x"}}}),
            )
            .await;
        connection
            .push(json!({"Ok": {"Ping": {"nonce": 1, "value": 0.0}}}))
            .await;
        assert_eq!(connection.next_frame().await, json!({"Pong": 1}));
        connection
            .push(json!({"Ok": {"Authenticated": user(ME)}}))
            .await;
        assert_eq!(connection.next_frame().await, json!("GetPendingGames"));
        assert_eq!(connection.next_frame().await, json!("Resync"));
        connection
            .push(json!({"Ok": {"Ping": {"nonce": 2, "value": 0.0}}}))
            .await;
        assert_eq!(connection.next_frame().await, json!({"Pong": 2}));
    }

    #[tokio::test]
    async fn plays_its_turns() {
        let hivegame = Hivegame::start().await;
        let mut connection = hivegame.authenticated().await;

        connection
            .push(json!({"Ok": {"Game": {"Urgent": [game("theirs", THEM)]}}}))
            .await;
        connection
            .push(json!({"Ok": {"Game": {"Reaction": {
                "game_action": "Join", "game": game("ours", ME),
                "game_id": "ours", "user_id": THEM, "username": "them"
            }}}}))
            .await;

        assert_eq!(
            connection.next_frame().await,
            json!({"Game": {"game_id": "ours", "action": {"Play": "bG1 -wS1"}}})
        );
    }

    #[tokio::test]
    async fn accepts_only_challenges_addressed_to_it() {
        let hivegame = Hivegame::start().await;
        let mut connection = hivegame.authenticated().await;

        connection
            .push(json!({"Ok": {"Challenge": {"Created": {"challenge_id": "open", "opponent": null}}}}))
            .await;
        connection
            .push(json!({"Ok": {"Challenge": {"Direct": {"challenge_id": "other", "opponent": user(THEM)}}}}))
            .await;
        connection
            .push(json!({"Ok": {"Challenge": {"Direct": {"challenge_id": "ours", "opponent": user(ME)}}}}))
            .await;

        assert_eq!(
            connection.next_frame().await,
            json!({"Challenge": {"Accept": "ours"}})
        );
    }

    #[tokio::test]
    async fn reconnects_after_a_refused_auth() {
        let hivegame = Hivegame::start().await;
        let mut refused = hivegame.accept().await;
        assert_eq!(refused.next_frame().await, json!({"Auth": "tok"}));
        refused.push(json!({"Ok": {"Error": "Auth failed"}})).await;

        hivegame.authenticated().await;
    }

    #[tokio::test]
    async fn replays_pending_turns_after_a_reconnect() {
        let hivegame = Hivegame::start().await;
        let mut first = hivegame.authenticated().await;
        first
            .push(json!({"Ok": {"Game": {"Urgent": [game("ours", ME)]}}}))
            .await;
        let play = json!({"Game": {"game_id": "ours", "action": {"Play": "bG1 -wS1"}}});
        assert_eq!(first.next_frame().await, play);
        drop(first);

        let mut second = hivegame.authenticated().await;
        second
            .push(json!({"Ok": {"Game": {"Urgent": [game("ours", ME)]}}}))
            .await;
        assert_eq!(second.next_frame().await, play);
    }
}
