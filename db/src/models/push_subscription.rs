use crate::{db_error::DbError, schema::push_subscriptions, DbConn};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = push_subscriptions)]
pub struct NewPushSubscription {
    pub user_id: Uuid,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
}

#[derive(Queryable, Identifiable, Clone, Debug, Selectable)]
#[diesel(table_name = push_subscriptions)]
#[diesel(primary_key(id))]
pub struct PushSubscription {
    pub id: Uuid,
    pub user_id: Uuid,
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub created_at: DateTime<Utc>,
}

impl PushSubscription {
    /// Store a subscription, refreshing the keys if this endpoint already
    /// exists (browsers reuse an endpoint and rotate its keys).
    pub async fn upsert(
        new: NewPushSubscription,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        Ok(diesel::insert_into(push_subscriptions::table)
            .values(&new)
            .on_conflict(push_subscriptions::endpoint)
            .do_update()
            .set((
                push_subscriptions::user_id.eq(&new.user_id),
                push_subscriptions::p256dh.eq(&new.p256dh),
                push_subscriptions::auth.eq(&new.auth),
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn find_by_user(
        user_id: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<Vec<Self>, DbError> {
        Ok(push_subscriptions::table
            .filter(push_subscriptions::user_id.eq(user_id))
            .select(Self::as_select())
            .get_results(conn)
            .await?)
    }

    pub async fn delete_by_endpoint(
        endpoint: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        Ok(diesel::delete(
            push_subscriptions::table.filter(push_subscriptions::endpoint.eq(endpoint)),
        )
        .execute(conn)
        .await?)
    }
}
