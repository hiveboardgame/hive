use crate::{
    common::ServerMessage,
    responses::UserResponse,
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, DbPool};
use uuid::Uuid;

pub struct UserProfileHandler {
    profile_username: String,
    user_id: Uuid,
    pool: DbPool,
}

impl UserProfileHandler {
    pub async fn new(user_id: Uuid, username: String, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            user_id,
            profile_username: username,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let user: UserResponse =
            UserResponse::from_username(&self.profile_username, &mut conn).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::PlayerProfile(user),
        }])
    }
}
