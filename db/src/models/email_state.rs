use crate::{db_error::DbError, schema::email_state::dsl, DbConn};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;

#[derive(Queryable, Debug, Clone)]
pub struct EmailState {
    pub id: i16,
    pub last_cleanup_run_at: Option<DateTime<Utc>>,
}

impl EmailState {
    pub async fn get(conn: &mut DbConn<'_>) -> Result<EmailState, DbError> {
        Ok(dsl::email_state.find(1_i16).first(conn).await?)
    }

    pub async fn set_cleanup_run(at: DateTime<Utc>, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::update(dsl::email_state.find(1_i16))
            .set(dsl::last_cleanup_run_at.eq(at))
            .execute(conn)
            .await?;
        Ok(())
    }
}
