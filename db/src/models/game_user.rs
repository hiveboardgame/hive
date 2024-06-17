use crate::{
    db_error::DbError,
    models::{Game, User},
    schema::games_users::{self, dsl::games_users as games_users_table},
    DbConn,
};
use diesel::{prelude::*, Identifiable, Insertable, Queryable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = user_id))]
#[diesel(belongs_to(Game))]
#[diesel(table_name = games_users)]
#[diesel(primary_key(game_id, user_id))]
pub struct GameUser {
    pub game_id: Uuid,
    pub user_id: Uuid,
}

impl GameUser {
    pub fn new(game_id: Uuid, user_id: Uuid) -> Self {
        Self { game_id, user_id }
    }

    pub async fn insert(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        self.insert_into(games_users_table).execute(conn).await?;
        Ok(())
    }
}
