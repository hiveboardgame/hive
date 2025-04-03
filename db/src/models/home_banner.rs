use crate::{db_error::DbError, schema::home_banner, DbConn};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Queryable, Identifiable, Clone, Debug, AsChangeset, Selectable)]
#[diesel(table_name = home_banner)]
#[diesel(primary_key(id))]
pub struct HomeBanner {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub display: bool,
}

impl HomeBanner {
    pub async fn get(conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        home_banner::table.first(conn).await.map_err(|e| e.into())
    }

    pub async fn update(self, conn: &mut DbConn<'_>) -> Result<usize, DbError> {
        diesel::update(home_banner::table.find(self.id))
            .set(self)
            .execute(conn)
            .await
            .map_err(|e| e.into())
    }
}
