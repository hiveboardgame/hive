use crate::{
    common::{
        GameReaction, {GameActionResponse, GameUpdate, ServerMessage},
    },
    responses::{GameResponse, UserResponse},
    websockets::{
        chat::Chats,
        internal_server_message::{InternalServerMessage, MessageDestination},
        messages::WsMessage,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Game, DbPool};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
use uuid::Uuid;

#[derive(Debug)]
pub struct JoinHandler {
    pool: DbPool,
    received_from: actix::Recipient<WsMessage>,
    chat_storage: actix_web::web::Data<Chats>,
    user_id: Uuid,
    username: String,
    game: Game,
}

impl JoinHandler {
    pub fn new(
        game: &Game,
        username: &str,
        user_id: Uuid,
        received_from: actix::Recipient<WsMessage>,
        chat_storage: actix_web::web::Data<Chats>,
        pool: &DbPool,
    ) -> Self {
        Self {
            received_from,
            game: game.to_owned(),
            user_id,
            username: username.to_owned(),
            chat_storage,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;

        let game = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move { Ok(Game::find_by_nanoid(&self.game.nanoid, tc).await?) }.scope_boxed()
            })
            .await?;

        let mut messages = Vec::new();
        if let Ok(user) = UserResponse::from_uuid(&self.user_id, &mut conn).await {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Game(game.nanoid.clone()),
                message: ServerMessage::Join(user),
            });
        } else {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Game(game.nanoid.clone()),
                message: ServerMessage::Join(UserResponse::for_anon(self.user_id)),
            });
        }
        messages.push(InternalServerMessage {
            destination: MessageDestination::Direct(self.received_from.clone()),
            message: ServerMessage::Game(Box::new(GameUpdate::Reaction(GameActionResponse {
                game_id: game.nanoid.to_owned(),
                game: GameResponse::new_from_db(&game, &mut conn).await?,
                game_action: GameReaction::Join,
                user_id: self.user_id.to_owned(),
                username: self.username.to_owned(),
            }))),
        });
        let games = if self.user_id == game.white_id || self.user_id == game.black_id {
            self.chat_storage.games_private.read().unwrap()
        } else {
            self.chat_storage.games_public.read().unwrap()
        };
        if let Some(messages_to_push) = games.get(&game.nanoid) {
            messages.push(InternalServerMessage {
                destination: MessageDestination::User(self.user_id),
                message: ServerMessage::Chat(messages_to_push.clone()),
            });
        };
        Ok(messages)
    }
}
