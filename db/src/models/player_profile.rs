use crate::{db_error::DbError, schema::player_profiles, DbConn};
use diesel::{
    AsChangeset,
    Insertable,
    OptionalExtension,
    QueryDsl,
    Queryable,
    Selectable,
    SelectableHelper,
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = player_profiles)]
#[diesel(treat_none_as_null = true)]
pub struct PlayerProfile {
    pub user_id: Uuid,
    pub flag: Option<String>,
    pub location: Option<String>,
    pub birth_year: Option<i32>,
    pub gender: Option<String>,
    pub bio: String,
}

impl PlayerProfile {
    pub async fn find(user_id: Uuid, conn: &mut DbConn<'_>) -> Result<Option<Self>, DbError> {
        Ok(player_profiles::table
            .find(user_id)
            .select(Self::as_select())
            .first(conn)
            .await
            .optional()?)
    }

    pub async fn save(&self, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::insert_into(player_profiles::table)
            .values(self)
            .on_conflict(player_profiles::user_id)
            .do_update()
            .set(self)
            .execute(conn)
            .await?;
        Ok(())
    }
}
