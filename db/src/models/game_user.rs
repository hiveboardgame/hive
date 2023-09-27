use crate::schema::{games_users, games_users::dsl::games_users as games_users_table};
use crate::models::{game::Game, user::User};
use crate::{DbPool, get_conn};
use diesel::{prelude::*, result::Error, Identifiable, Insertable, Queryable};
use diesel_async::RunQueryDsl;

#[derive(Insertable, Identifiable, Selectable, Queryable, Associations, Debug, Clone)]
#[diesel(belongs_to(User, foreign_key = user_uid))]
#[diesel(belongs_to(Game))]
#[diesel(table_name = games_users)]
#[diesel(primary_key(game_id, user_uid))]
pub struct GameUser {
    pub game_id: i32,
    pub user_uid: String,
}

impl GameUser {
    pub fn new(game_id: i32, user_uid: String) -> Self {
        Self { game_id, user_uid }
    }

    pub async fn insert(&self, pool: &DbPool) -> Result<(), Error> {
        let conn = &mut get_conn(pool).await?;
        self.insert_into(games_users_table).execute(conn).await?;
        Ok(())
    }
}
