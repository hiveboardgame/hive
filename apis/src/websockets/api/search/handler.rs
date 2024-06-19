use crate::{
    common::ServerMessage,
    responses::UserResponse,
    websockets::internal_server_message::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::User, DbPool};
use uuid::Uuid;

pub struct UserSearchHandler {
    user_id: Uuid,
    pattern: String,
    pool: DbPool,
}

impl UserSearchHandler {
    pub fn new(user_id: Uuid, pattern: String, pool: &DbPool) -> Self {
        Self {
            user_id,
            pattern,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let users = User::search_usernames(&self.pattern, &mut conn).await?;
        let mut response = vec![];
        for user in users {
            let user_response = UserResponse::from_model(&user, &mut conn).await?;
            response.push(user_response);
        }
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::UserSearch(response),
        }])
    }
}
