use std::sync::Arc;

use super::tournament_progression::{finish_post_commit_projection, settle_deadline};
use crate::{
    common::{GameActionResponse, GameReaction, GameUpdate, ServerMessage},
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        WebsocketData,
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use shared_types::GameId;
use uuid::Uuid;

pub struct JoinHandler {
    pool: DbPool,
    received_from: SocketTx,
    data: Arc<WebsocketData>,
    hub: Arc<WsHub>,
    user_id: Uuid,
    username: String,
    game_id: Uuid,
}

impl JoinHandler {
    pub fn new(
        game: &Game,
        username: &str,
        user_id: Uuid,
        received_from: SocketTx,
        data: Arc<WebsocketData>,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            received_from,
            game_id: game.id,
            user_id,
            username: username.to_owned(),
            data,
            hub,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let mut projected = settle_deadline(self.game_id, self.data.as_ref(), &mut conn).await?;
        if projected.removed {
            return Err(anyhow::anyhow!(
                "deadline check removed game {}",
                projected.game.nanoid
            ));
        }
        if let Some(error) = projected.rejected.take() {
            projected.output.request_error = Some(error.into());
            return Ok(projected.output);
        }
        let game = projected.game;
        let game_id = GameId(game.nanoid.clone());
        let projection = async {
            let game_response = self.data.get_or_build_response(&game, &mut conn).await?;
            install_join_membership(
                &self.hub,
                self.user_id,
                self.received_from.socket_id,
                &game_id,
                game.finished,
            );
            Ok(HandlerOutput::from(vec![
                InternalServerMessage {
                    destination: MessageDestination::Game(game_id.clone()),
                    message: ServerMessage::Join(self.user_id),
                },
                InternalServerMessage {
                    destination: MessageDestination::Direct(self.received_from.clone()),
                    message: ServerMessage::Game(Box::new(GameUpdate::Reaction(
                        GameActionResponse {
                            game_id: game_id.clone(),
                            game: (*game_response).clone(),
                            game_action: GameReaction::Join,
                            user_id: self.user_id,
                            username: self.username.clone(),
                        },
                    ))),
                },
            ]))
        }
        .await;
        Ok(finish_post_commit_projection(
            &game.nanoid,
            projected.output,
            projection,
        ))
    }
}

fn install_join_membership(
    hub: &WsHub,
    user_id: Uuid,
    socket_id: Uuid,
    game_id: &GameId,
    game_finished: bool,
) {
    hub.subscribe_game_fanout(user_id, socket_id, game_id);
    if !game_finished {
        hub.subscribe_game_heartbeat(user_id, socket_id, game_id);
    }
}

#[cfg(test)]
mod tests {
    use super::install_join_membership;
    use crate::websocket::{WebsocketData, WsHub};
    use db_lib::get_pool;
    use shared_types::GameId;
    use std::sync::Arc;
    use uuid::Uuid;

    #[tokio::test]
    async fn join_membership_distinguishes_live_and_finished_games() {
        let pool = get_pool("postgresql://test:test@127.0.0.1:9/test")
            .await
            .expect("bb8 pool builds without connecting");
        let hub = WsHub::new(Arc::new(WebsocketData::default()), pool);
        let user_id = Uuid::new_v4();
        let socket_id = Uuid::new_v4();
        let live_game = GameId("live-join".to_string());
        let finished_game = GameId("finished-join".to_string());

        install_join_membership(&hub, user_id, socket_id, &live_game, false);
        install_join_membership(&hub, user_id, socket_id, &finished_game, true);

        assert!(hub.has_game_fanout(user_id, socket_id, &live_game));
        assert!(hub.has_game_heartbeat(user_id, socket_id, &live_game));
        assert!(hub.has_game_fanout(user_id, socket_id, &finished_game));
        assert!(!hub.has_game_heartbeat(user_id, socket_id, &finished_game));
    }
}
