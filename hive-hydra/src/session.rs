use crate::{
    bot::{self, Turn},
    config::BotConfig,
    protocol::{self, Challenge, ChallengeAction, ClientRequest, Game, Incoming, User},
    token::{self, TokenError},
    turn_tracker::{TurnTracker, TurnTracking},
};
use futures_util::{SinkExt, StreamExt};
use std::{pin::pin, sync::Arc, time::Duration};
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

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
// The server sends a protocol ping every 5 s, so this much silence means the connection is dead.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(15);
// Pushes cover every move and challenge; this retries turns whose engine run failed and
// challenges whose accept was refused.
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
    #[error("no authenticated connection within {CONNECT_TIMEOUT:?}")]
    ConnectTimeout,
    #[error("connection closed during authentication")]
    ClosedDuringAuth,
    #[error("nothing heard from the server for {SILENCE_TIMEOUT:?}")]
    Silent,
}

pub enum Outgoing {
    // Counts as played only once written, so the reconnect, which forgets played positions,
    // keeps tracking a move still waiting here.
    Move {
        position_key: u64,
        request: ClientRequest,
    },
    Resync,
}

pub struct Shared {
    pub http: reqwest::Client,
    pub base_url: String,
    pub websocket_url: String,
    pub engine_slots: Semaphore,
}

struct Session {
    bot: Arc<BotConfig>,
    shared: Arc<Shared>,
    me: User,
    outbox: mpsc::UnboundedSender<Outgoing>,
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
    timeout(
        CONNECT_TIMEOUT,
        authenticated_socket(&shared.websocket_url, token),
    )
    .await
    .map_err(|_| SessionError::ConnectTimeout)?
}

async fn authenticated_socket(url: &str, token: String) -> Result<(Socket, User), SessionError> {
    // A game list is a single frame and runs to megabytes for a bot with many games.
    let config = WebSocketConfig::default()
        .max_message_size(None)
        .max_frame_size(None);
    let (mut socket, _) = connect_async_with_config(url, Some(config), true).await?;
    send(&mut socket, &ClientRequest::Auth(token)).await?;
    let me = await_authentication(&mut socket).await?;
    Ok((socket, me))
}

async fn await_authentication(socket: &mut Socket) -> Result<User, SessionError> {
    while let Some(frame) = socket.next().await {
        let Message::Text(text) = frame? else {
            continue;
        };
        match protocol::parse(&text) {
            Ok(Incoming::Authenticated(me)) => return Ok(me),
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
        queued: &mut mpsc::UnboundedReceiver<Outgoing>,
    ) -> Result<(), SessionError> {
        // The server handles a socket's requests in order, so moves that finished while we were
        // disconnected go first rather than wait behind the snapshot query.
        while let Ok(outgoing) = queued.try_recv() {
            self.deliver(&mut socket, outgoing).await?;
        }
        send(&mut socket, &ClientRequest::Resync).await?;
        let mut reconcile = interval_at(Instant::now() + RECONCILE_INTERVAL, RECONCILE_INTERVAL);
        reconcile.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut silence = pin!(sleep(SILENCE_TIMEOUT));
        loop {
            tokio::select! {
                frame = socket.next() => {
                    silence.as_mut().reset(Instant::now() + SILENCE_TIMEOUT);
                    match frame {
                        None => return Ok(()),
                        Some(frame) => {
                            if let Message::Text(text) = frame? {
                                self.handle(&mut socket, &text).await?;
                            }
                        }
                    }
                }
                Some(outgoing) = queued.recv() => self.deliver(&mut socket, outgoing).await?,
                _ = reconcile.tick() => {
                    self.turn_tracker.cleanup().await;
                    send(&mut socket, &ClientRequest::Resync).await?;
                }
                () = &mut silence => return Err(SessionError::Silent),
            }
        }
    }

    async fn deliver(&self, socket: &mut Socket, outgoing: Outgoing) -> Result<(), SessionError> {
        match outgoing {
            Outgoing::Move {
                position_key,
                request,
            } => {
                // Marked before the write: if it fails, the reconnect forgets the position and
                // the snapshot replays the turn.
                self.turn_tracker.processed(position_key).await;
                send(socket, &request).await
            }
            Outgoing::Resync => send(socket, &ClientRequest::Resync).await,
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
                self.shared.clone(),
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
            let _ = outbox.send(Outgoing::Resync);
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
    const ENGINE: &str =
        "cat > /dev/null\nprintf 'id fake\\nok\\nBase+MLP;InProgress;Black[1];wS1\\nok\\nbG1 -wS1\\nok\\n'\n";

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
            Self::with_engine(ENGINE).await
        }

        async fn with_engine(script: &str) -> Self {
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
            fs::write(&engine, script).unwrap();
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
                engine_slots: Semaphore::new(1),
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
            "game_type": "MLP", "history": [["wS1", ""]], "turn": 1,
            "game_start": "Moves", "game_status": "InProgress"
        })
    }

    fn our_move() -> Value {
        json!({"Game": {"game_id": "ours", "action": {"PlayAt": {"turn": 1, "notation": "bG1 -wS1"}}}})
    }

    fn reaction(game_action: Value, game: Value) -> Value {
        json!({"Ok": {"Game": {"Reaction": {
            "game_action": game_action, "game_id": game["game_id"], "game": game,
            "user_id": THEM, "username": "them"
        }}}})
    }

    #[tokio::test]
    async fn leaves_pings_unanswered() {
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
        connection
            .push(json!({"Ok": {"Authenticated": user(ME)}}))
            .await;
        assert_eq!(connection.next_frame().await, json!("Resync"));
        connection
            .push(json!({"Ok": {"Ping": {"nonce": 2, "value": 0.0}}}))
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
    async fn plays_its_turns() {
        let hivegame = Hivegame::start().await;
        let mut connection = hivegame.authenticated().await;

        connection
            .push(json!({"Ok": {"Game": {"Urgent": [game("theirs", THEM)]}}}))
            .await;
        connection
            .push(reaction(json!("Join"), game("ours", ME)))
            .await;

        assert_eq!(connection.next_frame().await, our_move());
    }

    #[tokio::test]
    async fn waits_for_a_ready_game_to_start() {
        let hivegame = Hivegame::start().await;
        let mut connection = hivegame.authenticated().await;
        let mut released = game("ours", ME);
        released["game_start"] = json!("Ready");
        released["game_status"] = json!("NotStarted");

        connection
            .push(reaction(
                json!({"New": {"pairing_label": null}}),
                released.clone(),
            ))
            .await;
        assert!(
            timeout(Duration::from_millis(500), connection.socket.next())
                .await
                .is_err(),
            "hydra moved before the game started"
        );

        let mut started = released;
        started["game_status"] = json!("InProgress");
        connection.push(reaction(json!("Started"), started)).await;
        assert_eq!(connection.next_frame().await, our_move());
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
        assert_eq!(first.next_frame().await, our_move());
        drop(first);

        let mut second = hivegame.authenticated().await;
        second
            .push(json!({"Ok": {"Game": {"Urgent": [game("ours", ME)]}}}))
            .await;
        assert_eq!(second.next_frame().await, our_move());
    }

    #[tokio::test]
    async fn sends_a_move_finished_while_disconnected_first_and_once() {
        // Slower than noticing the hang-up, faster than the reconnect delay.
        let hivegame = Hivegame::with_engine(&format!("sleep 0.3\n{ENGINE}")).await;
        let mut first = hivegame.authenticated().await;
        first
            .push(json!({"Ok": {"Game": {"Urgent": [game("ours", ME)]}}}))
            .await;
        drop(first);

        let mut second = hivegame.accept().await;
        assert_eq!(second.next_frame().await, json!({"Auth": "tok"}));
        second
            .push(json!({"Ok": {"Authenticated": user(ME)}}))
            .await;
        assert_eq!(second.next_frame().await, our_move());
        assert_eq!(second.next_frame().await, json!("Resync"));

        // Sent before the server saw the move.
        second
            .push(json!({"Ok": {"Game": {"Urgent": [game("ours", ME)]}}}))
            .await;
        assert!(
            timeout(Duration::from_secs(1), second.socket.next())
                .await
                .is_err(),
            "hydra played the same turn twice"
        );
    }
}
