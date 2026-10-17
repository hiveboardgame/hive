use super::projection::settle_deadline;
use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        WebsocketData,
        WsHub,
    },
};
use anyhow::Result;
use chrono::Utc;
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
    received_from: SocketTx,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
    pool: DbPool,
}

impl TimeoutHandler {
    pub fn new(
        game: &Game,
        received_from: SocketTx,
        data: Arc<WebsocketData>,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game_id: game.id,
            received_from,
            data,
            hub,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let mut projected = settle_deadline(self.game_id, self.data.as_ref(), &mut conn).await?;
        if !projected.game.finished {
            self.settle_after_the_grace(&projected.game);
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

impl TimeoutHandler {
    // A clock that reads zero is not flagged until its grace has passed, and the client asks only
    // once, so check again when the grace ends, and keep following the game while the next flag
    // is that close.
    fn settle_after_the_grace(&self, game: &Game) {
        let Some(wait) = recheck_wait(game) else {
            return;
        };
        if !self.hub.flag_rechecks.insert(self.game_id) {
            return;
        }
        let recheck = Recheck {
            hub: Arc::clone(&self.hub),
            game_id: self.game_id,
        };
        actix_rt::spawn(async move {
            let mut wait = wait;
            loop {
                tokio::time::sleep(wait.max(MIN_RECHECK_GAP)).await;
                let Some(game) = recheck.settle().await else {
                    return;
                };
                match recheck_wait(&game) {
                    Some(next) if !game.finished => wait = next,
                    _ => return,
                }
            }
        });
    }
}

fn recheck_wait(game: &Game) -> Option<Duration> {
    let wait = (game.flag_due_at()? - Utc::now())
        .to_std()
        .unwrap_or_default();
    (wait <= MAX_RECHECK_WAIT).then_some(wait)
}

struct Recheck {
    hub: Arc<WsHub>,
    game_id: Uuid,
}

impl Recheck {
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
        self.hub.flag_rechecks.remove(&self.game_id);
    }
}
