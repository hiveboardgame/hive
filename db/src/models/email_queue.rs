use crate::{
    db_error::DbError,
    schema::email_queue::{
        self,
        dsl::{
            attempts as attempts_field,
            created_at,
            email_queue as email_queue_table,
            id as id_field,
            last_error as last_error_field,
            scheduled_at,
            sent_at,
        },
    },
    DbConn,
};
use chrono::{DateTime, Utc};
use diesel::{ExpressionMethods, Insertable, QueryDsl, Queryable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = email_queue)]
pub struct NewEmailQueueItem {
    pub user_id: Option<Uuid>,
    pub kind: String,
    pub payload: serde_json::Value,
    pub to_address: String,
}

#[derive(Queryable, Debug, Clone)]
pub struct EmailQueueItem {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub kind: String,
    pub payload: serde_json::Value,
    pub to_address: String,
    pub created_at: DateTime<Utc>,
    pub scheduled_at: DateTime<Utc>,
    pub attempts: i16,
    pub last_error: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
}

impl EmailQueueItem {
    pub async fn enqueue(
        new: NewEmailQueueItem,
        conn: &mut DbConn<'_>,
    ) -> Result<EmailQueueItem, DbError> {
        Ok(diesel::insert_into(email_queue_table)
            .values(new)
            .get_result(conn)
            .await?)
    }

    pub async fn claim_batch(
        limit: i64,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<EmailQueueItem>, DbError> {
        Ok(email_queue_table
            .filter(sent_at.is_null())
            .filter(attempts_field.lt(3))
            .filter(scheduled_at.le(Utc::now()))
            .order(created_at.asc())
            .limit(limit)
            .load(conn)
            .await?)
    }

    pub async fn mark_sent(id: Uuid, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::update(email_queue_table.filter(id_field.eq(id)))
            .set(sent_at.eq(Utc::now()))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn mark_skipped(id: Uuid, note: &str, conn: &mut DbConn<'_>) -> Result<(), DbError> {
        diesel::update(email_queue_table.filter(id_field.eq(id)))
            .set((sent_at.eq(Utc::now()), last_error_field.eq(note.to_owned())))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn mark_failed(
        id: Uuid,
        attempts: i16,
        last_error: &str,
        next_at: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        diesel::update(email_queue_table.filter(id_field.eq(id)))
            .set((
                attempts_field.eq(attempts),
                last_error_field.eq(last_error.to_owned()),
                scheduled_at.eq(next_at),
            ))
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn prune_sent(
        threshold: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        Ok(diesel::delete(
            email_queue_table
                .filter(sent_at.is_not_null())
                .filter(sent_at.lt(threshold)),
        )
        .execute(conn)
        .await?)
    }

    pub async fn prune_failed(
        threshold: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        Ok(diesel::delete(
            email_queue_table
                .filter(sent_at.is_null())
                .filter(attempts_field.ge(3))
                .filter(created_at.lt(threshold)),
        )
        .execute(conn)
        .await?)
    }
}
