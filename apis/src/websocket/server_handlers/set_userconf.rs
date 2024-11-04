use crate::common::ServerMessage;
use crate::websocket::messages::InternalServerMessage;
use crate::websocket::messages::MessageDestination;
use anyhow::Result;
use db_lib::{get_conn, models::User, DbPool};
use shared_types::Takeback;
use uuid::Uuid;
pub struct ServerUserConfHandler {
    takeback: Takeback,
    user_id: Uuid,
    pool: DbPool,
}

impl ServerUserConfHandler {
    pub async fn new(user_id: Uuid, takeback: Takeback, pool: &DbPool) -> Result<Self> {
        Ok(Self {
            user_id,
            takeback,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;

        let user = User::find_by_uuid(&self.user_id, &mut conn).await?;
        user.set_takeback(self.takeback.clone(), &mut conn).await?;
        Ok(vec![InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::CouldSetUserConf(true),
        }])
    }
}
