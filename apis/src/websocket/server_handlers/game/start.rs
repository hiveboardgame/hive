use super::tournament_progression::{project_committed_game, GameCommandContext, ProjectedGame};
use crate::{
    common::GameReaction,
    websocket::{messages::HandlerOutput, WebsocketData},
};
use anyhow::{anyhow, Result};
use db_lib::{
    game_command::{self, Command},
    get_conn,
    models::{Game, User},
    DbConn,
    DbPool,
};
use shared_types::GameId;
use std::sync::Arc;
use uuid::Uuid;

pub struct StartHandler {
    pool: DbPool,
    data: Arc<WebsocketData>,
    user_id: Uuid,
    username: String,
    game_id: Uuid,
    nanoid: String,
}

impl StartHandler {
    pub fn new(
        game: &Game,
        user_id: Uuid,
        username: String,
        data: Arc<WebsocketData>,
        pool: &DbPool,
    ) -> Self {
        Self {
            game_id: game.id,
            nanoid: game.nanoid.clone(),
            user_id,
            username,
            data,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let mut ready = self
            .ready_candidate(GameCommandContext::silent(), &mut conn)
            .await?;
        let ready_id = GameId(ready.game.nanoid.clone());
        if ready.removed {
            self.data.game_start.forget(&ready_id);
            return Err(anyhow!("Ready intent removed game {}", ready.game.nanoid));
        }
        if let Some(error) = ready.rejected.take() {
            self.data.game_start.forget(&ready_id);
            ready.output.request_error = Some(error.into());
            return Ok(ready.output);
        }

        let opponent_id = if ready.game.white_id == self.user_id {
            ready.game.black_id
        } else {
            ready.game.white_id
        };
        let opponent_is_bot = match User::find_by_uuid(&opponent_id, &mut conn).await {
            Ok(opponent) => opponent.bot,
            Err(error) => {
                self.data.game_start.forget(&ready_id);
                return Err(error.into());
            }
        };
        let should_start = if opponent_is_bot {
            Ok(true)
        } else {
            self.data.game_start.should_start(&ready.game, self.user_id)
        };
        let should_start = match should_start {
            Ok(should_start) => should_start,
            Err(error) => {
                self.data.game_start.forget(&ready_id);
                return Err(error);
            }
        };

        if should_start {
            let outcome = match game_command::execute(
                ready.game.id,
                Command::StartReady {
                    user_id: self.user_id,
                },
                &mut conn,
            )
            .await
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    self.data.game_start.forget(&ready_id);
                    return Err(error.into());
                }
            };
            self.data.game_start.forget(&ready_id);
            let context = GameCommandContext::websocket(
                GameReaction::Started,
                self.user_id,
                self.username.clone(),
            );
            let mut started =
                project_committed_game(outcome, context, self.data.as_ref(), &mut conn).await;
            ready.output.append(started.output);
            if started.removed {
                ready.output.request_error =
                    Some(anyhow!("Ready start removed game {}", started.game.nanoid));
                return Ok(ready.output);
            }
            if let Some(error) = started.rejected.take() {
                ready.output.request_error = Some(error.into());
                return Ok(ready.output);
            }
            return Ok(ready.output);
        }

        let context =
            GameCommandContext::websocket(GameReaction::Ready, self.user_id, self.username.clone());
        let mut candidate = self.ready_candidate(context, &mut conn).await?;
        ready.output.append(candidate.output);
        if candidate.removed {
            self.data.game_start.forget(&ready_id);
            ready.output.request_error = Some(anyhow!(
                "Ready intent removed game {}",
                candidate.game.nanoid
            ));
            return Ok(ready.output);
        }
        if let Some(error) = candidate.rejected.take() {
            self.data.game_start.forget(&ready_id);
            ready.output.request_error = Some(error.into());
            return Ok(ready.output);
        }
        Ok(ready.output)
    }

    async fn ready_candidate(
        &self,
        context: GameCommandContext,
        conn: &mut DbConn<'_>,
    ) -> Result<ProjectedGame> {
        let ready_id = GameId(self.nanoid.clone());
        let outcome = match game_command::execute(
            self.game_id,
            Command::ReadyIntent {
                user_id: self.user_id,
            },
            conn,
        )
        .await
        {
            Ok(outcome) => outcome,
            Err(error) => {
                self.data.game_start.forget(&ready_id);
                return Err(error.into());
            }
        };
        Ok(project_committed_game(outcome, context, self.data.as_ref(), conn).await)
    }
}
