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
use chrono::{DateTime, Duration, Utc};
use diesel::{ExpressionMethods, Insertable, QueryDsl, Queryable};
use diesel_async::{AsyncConnection, RunQueryDsl};
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

    /// Takes a batch out of the queue and leases it to this caller.
    ///
    /// A plain `SELECT` is not enough: the drain delivers before it marks
    /// anything, so two app instances — which every blue/green deploy has for
    /// the length of the overlap window — read the same rows and both send.
    /// Pushing `scheduled_at` forward in the same transaction that locks the
    /// rows is the lease: the other instance's `scheduled_at <= now()` filter
    /// stops matching them. `SKIP LOCKED` is what keeps the two from simply
    /// queueing behind each other and handing over the same batch anyway.
    ///
    /// `attempts` is deliberately not bumped: an instance that dies mid-batch
    /// should have the rest retried in full once the lease expires, not have
    /// them burn a delivery attempt that was never made.
    ///
    /// `lease` is the caller's, because only the caller knows its worst-case
    /// time to work through `limit` items. A lease shorter than that lets a
    /// second instance reclaim and re-send messages the first is still sending.
    pub async fn claim_batch(
        limit: i64,
        lease: Duration,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<EmailQueueItem>, DbError> {
        let lease = Utc::now() + lease;
        conn.transaction::<_, DbError, _>(async move |tc| {
            let ids: Vec<Uuid> = email_queue_table
                .select(id_field)
                .filter(sent_at.is_null())
                .filter(attempts_field.lt(3))
                .filter(scheduled_at.le(Utc::now()))
                .order(created_at.asc())
                .limit(limit)
                .for_update()
                .skip_locked()
                .load(tc)
                .await?;
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            Ok(
                diesel::update(email_queue_table.filter(id_field.eq_any(ids)))
                    .set(scheduled_at.eq(lease))
                    .get_results(tc)
                    .await?,
            )
        })
        .await
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
