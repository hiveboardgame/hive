use actix::prelude::*;
use uuid::Uuid;
use crate::common::{game_action::GameAction, server_message::ServerMessage};
use db_lib::DbPool;
use leptos::*;

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub game_id: String,
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct ClientActorMessage {
    pub game_action: GameAction,
    pub game_id: String,
    pub serialized: String,
    pub user_id: Uuid,
    pub username: String,
}

impl ClientActorMessage {
    pub async fn new(
        game_action: GameAction,
        game_id: &str,
        user_id: Uuid,
        username: &str,
        pool: &DbPool,
    ) -> Result<Self, ServerFnError> {
        let server_message = ServerMessage::new(&game_id, game_action.clone(), &user_id, &username, pool).await?;
        let serialized = serde_json::to_string(&server_message)?;
        Ok(Self {
            game_action,
            game_id: game_id.to_owned(),
            serialized,
            user_id,
            username: username.to_owned(),
        })
    }
}
