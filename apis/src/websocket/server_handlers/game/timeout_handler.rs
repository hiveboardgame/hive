use super::projection::{check_time, settle_deadline};
use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        WebsocketData,
        WsHub,
    },
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use db_lib::{get_conn, models::Game, DbPool};
use hive_lib::GameStatus;
use shared_types::{Conclusion, GameId};
use std::{str::FromStr, sync::Arc, time::Duration};
use uuid::Uuid;

// A client whose clock runs far ahead of ours is left to the game heartbeat.
const MAX_RECHECK_WAIT: Duration = Duration::from_secs(5);
const MIN_RECHECK_GAP: Duration = Duration::from_millis(100);

pub struct TimeoutHandler {
    game_id: Uuid,
    user_id: Uuid,
    turn: Option<i32>,
    received_from: SocketTx,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
    pool: DbPool,
}

impl TimeoutHandler {
    pub fn new(
        game: &Game,
        user_id: Uuid,
        turn: Option<i32>,
        received_from: SocketTx,
        data: Arc<WebsocketData>,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game_id: game.id,
            user_id,
            turn,
            received_from,
            data,
            hub,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        // A move of theirs from another tab or device could still be on its way.
        let own_check = self
            .turn
            .filter(|_| self.hub.socket_count(self.user_id) == 1);
        let mut projected = match own_check {
            Some(turn) => {
                check_time(
                    self.game_id,
                    self.user_id,
                    turn,
                    self.data.as_ref(),
                    &mut conn,
                )
                .await?
            }
            None => settle_deadline(self.game_id, self.data.as_ref(), &mut conn).await?,
        };
        if !projected.game.finished {
            settle_after_the_grace(&self.hub, &projected.game);
        }
        if projected.game.finished && projected.output.finalize_games.is_empty() {
            let game = self
                .data
                .get_or_build_response(&projected.game, &mut conn)
                .await?;
            let reaction = if projected.game.conclusion == Conclusion::Timeout.to_string() {
                GameReaction::TimedOut
            } else if matches!(
                GameStatus::from_str(&projected.game.game_status),
                Ok(GameStatus::Adjudicated)
            ) {
                GameReaction::Adjudicated
            } else {
                GameReaction::Finished
            };
            projected.output.messages.push(InternalServerMessage {
                destination: MessageDestination::Direct(self.received_from.clone()),
                message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                    game_id: GameId(projected.game.nanoid.clone()),
                    game: game.clone(),
                    game_action: reaction,
                    user_id: projected.game.current_player_id,
                    username: String::new(),
                }))),
            });
        }
        Ok(projected.output)
    }
}

// A clock that reads zero is not flagged until its grace has passed, and no client may be left to
// ask again, so check again when the grace ends. A flag still due afterwards waits on a received
// move, which may yet be rejected; each check locks the row that move waits for, so they back off.
fn settle_after_the_grace(hub: &Arc<WsHub>, game: &Game) {
    let Some(due) = game.flag_due_at() else {
        return;
    };
    let Some((recheck, wait)) = Recheck::claim(hub, game.id, due, Utc::now()) else {
        return;
    };
    actix_rt::spawn(async move {
        tokio::time::sleep(wait.max(MIN_RECHECK_GAP)).await;
        let Some(game) = recheck.settle().await else {
            return;
        };
        let hub = Arc::clone(&recheck.hub);
        drop(recheck);
        if !game.finished {
            settle_after_the_grace(&hub, &game);
        }
    });
}

struct Recheck {
    hub: Arc<WsHub>,
    game_id: Uuid,
    due: DateTime<Utc>,
}

impl Recheck {
    fn claim(
        hub: &Arc<WsHub>,
        game_id: Uuid,
        due: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Option<(Self, Duration)> {
        let wait = (due - now).to_std().unwrap_or_default();
        let overdue = (now - due).to_std().unwrap_or_default();
        if wait > MAX_RECHECK_WAIT
            || overdue > MAX_RECHECK_WAIT
            || !hub.flag_rechecks.insert((game_id, due))
        {
            return None;
        }
        let recheck = Self {
            hub: Arc::clone(hub),
            game_id,
            due,
        };
        let backoff = overdue.min(MAX_RECHECK_WAIT.saturating_sub(overdue));
        Some((recheck, wait.max(backoff)))
    }

    async fn settle(&self) -> Option<Game> {
        let settled = match get_conn(&self.hub.pool).await {
            Ok(mut conn) => settle_deadline(self.game_id, self.hub.data.as_ref(), &mut conn)
                .await
                .map_err(anyhow::Error::from),
            Err(error) => Err(error.into()),
        };
        match settled {
            Ok(projected) => {
                self.hub.dispatch_handler_output(projected.output).await;
                Some(projected.game)
            }
            Err(error) => {
                log::error!(
                    "settling game {} after its flag grace: {error}",
                    self.game_id
                );
                None
            }
        }
    }
}

impl Drop for Recheck {
    fn drop(&mut self) {
        self.hub.flag_rechecks.remove(&(self.game_id, self.due));
    }
}

#[cfg(test)]
mod tests {
    use super::{Recheck, MAX_RECHECK_WAIT};
    use crate::websocket::{WebsocketData, WsHub};
    use chrono::{Duration as ChronoDuration, Utc};
    use std::{sync::Arc, time::Duration};
    use uuid::Uuid;

    async fn hub() -> Arc<WsHub> {
        let pool = db_lib::get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting");
        WsHub::new(Arc::new(WebsocketData::default()), pool)
    }

    #[tokio::test]
    async fn a_sooner_flag_is_rechecked_while_a_later_one_waits() {
        let hub = hub().await;
        let (game_id, now) = (Uuid::new_v4(), Utc::now());
        let later = now + ChronoDuration::seconds(4);
        let sooner = now + ChronoDuration::seconds(1);

        let waiting = Recheck::claim(&hub, game_id, later, now).expect("nothing scheduled yet");
        assert!(
            Recheck::claim(&hub, game_id, later, now).is_none(),
            "a second CheckTime for the same flag schedules nothing"
        );
        let (_sooner, wait) =
            Recheck::claim(&hub, game_id, sooner, now).expect("a sooner flag gets its own check");
        assert_eq!(wait, Duration::from_secs(1));

        drop(waiting);
        assert!(
            Recheck::claim(&hub, game_id, later, now).is_some(),
            "a finished check frees its flag"
        );
    }

    #[tokio::test]
    async fn a_flag_further_off_than_the_longest_wait_is_left_to_the_heartbeat() {
        let hub = hub().await;
        let now = Utc::now();
        let due = now
            + ChronoDuration::from_std(MAX_RECHECK_WAIT).expect("fits")
            + ChronoDuration::milliseconds(1);

        assert!(Recheck::claim(&hub, Uuid::new_v4(), due, now).is_none());
    }

    #[tokio::test]
    async fn a_flag_held_back_by_a_received_move_is_retried_only_within_the_longest_wait() {
        let hub = hub().await;
        let now = Utc::now();
        let longest = ChronoDuration::from_std(MAX_RECHECK_WAIT).expect("fits");

        let (_retry, wait) = Recheck::claim(&hub, Uuid::new_v4(), now - longest, now)
            .expect("an overdue flag is checked again");
        assert_eq!(wait, Duration::ZERO);
        let long_overdue = now - longest - ChronoDuration::milliseconds(1);
        assert!(Recheck::claim(&hub, Uuid::new_v4(), long_overdue, now).is_none());
    }

    #[tokio::test]
    async fn an_overdue_flag_waits_as_long_again_until_the_longest_wait() {
        let hub = hub().await;
        let now = Utc::now();
        let wait_when = |overdue_ms| {
            Recheck::claim(
                &hub,
                Uuid::new_v4(),
                now - ChronoDuration::milliseconds(overdue_ms),
                now,
            )
            .expect("an overdue flag is checked again")
            .1
        };

        assert_eq!(wait_when(400), Duration::from_millis(400));
        assert_eq!(wait_when(3200), Duration::from_millis(1800));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_flagged_players_only_tab_ends_the_grace() {
        use super::TimeoutHandler;
        use crate::websocket::{
            messages::{SocketFormat, SocketTx},
            test_db::test_db,
            ws_hub::SOCKET_BUFFER_CAPACITY,
        };
        use db_lib::{
            get_conn,
            models::{Game, NewGame, NewUser, User},
        };
        use hive_lib::{GameStatus, GameType};
        use shared_types::{
            Conclusion,
            GameSpeed,
            GameStart,
            SimpleUser,
            TimeMode,
            TournamentGameResult,
        };

        let db = test_db().await;
        let pool = db.pool.clone();
        tokio::task::LocalSet::new()
            .run_until(async move {
                let mut conn = get_conn(&pool).await.expect("test database connection");
                let mut players = Vec::new();
                for name in ["ownflagw", "ownflagb"] {
                    let new_user = NewUser::new(name, "password", &format!("{name}@example.com"))
                        .expect("valid user");
                    players.push(User::create(new_user, &mut conn).await.expect("user"));
                }
                let (white, black) = (&players[0], &players[1]);
                let started = Utc::now() - ChronoDuration::milliseconds(60_100);
                let game = Game::create(
                    NewGame {
                        nanoid: Uuid::new_v4().simple().to_string()[..12].to_string(),
                        current_player_id: white.id,
                        black_id: black.id,
                        finished: false,
                        game_status: GameStatus::InProgress.to_string(),
                        game_type: GameType::MLP.to_string(),
                        history: "wQ;bQ wQ-;".to_string(),
                        game_control_history: String::new(),
                        rated: false,
                        tournament_queen_rule: false,
                        turn: 2,
                        white_id: white.id,
                        white_rating: None,
                        black_rating: None,
                        white_rating_change: None,
                        black_rating_change: None,
                        created_at: started,
                        updated_at: started,
                        time_mode: TimeMode::RealTime.to_string(),
                        time_base: Some(60),
                        time_increment: Some(0),
                        last_interaction: Some(started),
                        black_time_left: Some(60_000_000_000),
                        white_time_left: Some(60_000_000_000),
                        speed: GameSpeed::Bullet.to_string(),
                        hashes: Vec::new(),
                        conclusion: Conclusion::Unknown.to_string(),
                        tournament_id: None,
                        tournament_game_result: TournamentGameResult::Unknown.to_string(),
                        game_start: GameStart::Moves.to_string(),
                        move_times: Vec::new(),
                        timeout_at: Some(started + ChronoDuration::seconds(60)),
                        white_berserked: false,
                        black_berserked: false,
                        arena_move_due_at: None,
                        tournament_slot_id: None,
                        arena_ordinal: None,
                    },
                    &mut conn,
                )
                .await
                .expect("the game is created");
                drop(conn);

                let data = Arc::new(WebsocketData::default());
                let hub = WsHub::new(Arc::clone(&data), pool.clone());
                let identity = SimpleUser {
                    user_id: white.id,
                    username: white.username.clone(),
                    admin: false,
                    authed: true,
                };
                let mut receivers = Vec::new();
                let mut sockets = Vec::new();
                for _ in 0..2 {
                    let (tx, rx) = tokio::sync::mpsc::channel(SOCKET_BUFFER_CAPACITY);
                    let socket = SocketTx {
                        socket_id: Uuid::new_v4(),
                        format: SocketFormat::Msgpack,
                        tx,
                    };
                    Arc::clone(&hub).on_connect(
                        socket.socket_id,
                        socket.tx.clone(),
                        socket.format,
                        identity.clone(),
                    );
                    receivers.push(rx);
                    sockets.push(socket);
                }
                let check = |socket: &SocketTx| {
                    TimeoutHandler::new(
                        &game,
                        white.id,
                        Some(game.turn),
                        socket.clone(),
                        Arc::clone(&data),
                        Arc::clone(&hub),
                        &pool,
                    )
                };
                let conclusion = || async {
                    let mut conn = get_conn(&pool).await.expect("test database connection");
                    Game::find_by_uuid(&game.id, &mut conn)
                        .await
                        .expect("reload the game")
                        .conclusion
                };

                check(&sockets[0]).handle().await.expect("the check runs");
                assert_eq!(
                    conclusion().await,
                    Conclusion::Unknown.to_string(),
                    "a move from white's other device may still be on its way"
                );

                hub.on_disconnect(sockets[1].socket_id, identity.clone());
                check(&sockets[0]).handle().await.expect("the check runs");
                assert_eq!(conclusion().await, Conclusion::Timeout.to_string());
            })
            .await;
    }
}
