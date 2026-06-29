use crate::{db_error::DbError, schema::push_devices, DbConn};
use chrono::{DateTime, Utc};
use diesel::{
    upsert::excluded,
    ExpressionMethods,
    Identifiable,
    Insertable,
    QueryDsl,
    Queryable,
    Selectable,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = push_devices)]
pub struct NewPushDevice {
    pub user_id: Uuid,
    pub platform: String,
    pub device_token: String,
    pub app_version: String,
    pub locale: String,
    pub p256dh: Option<String>,
    pub auth: Option<String>,
}

#[derive(Queryable, Identifiable, Selectable, Serialize, Deserialize, Debug, Clone)]
#[diesel(primary_key(id))]
#[diesel(table_name = push_devices)]
pub struct PushDevice {
    pub id: Uuid,
    pub user_id: Uuid,
    pub platform: String,
    pub device_token: String,
    pub app_version: String,
    pub locale: String,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub p256dh: Option<String>,
    pub auth: Option<String>,
}

impl PushDevice {
    pub async fn upsert(
        new: NewPushDevice,
        clear_revocation: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        use crate::schema::push_devices::dsl::*;
        let now = Utc::now();
        let insert = diesel::insert_into(push_devices)
            .values(&new)
            .on_conflict((platform, device_token));
        let device = if clear_revocation {
            insert
                .do_update()
                .set((
                    user_id.eq(excluded(user_id)),
                    app_version.eq(excluded(app_version)),
                    locale.eq(excluded(locale)),
                    last_seen_at.eq(now),
                    p256dh.eq(excluded(p256dh)),
                    auth.eq(excluded(auth)),
                    revoked_at.eq(None::<DateTime<Utc>>),
                ))
                .get_result(conn)
                .await?
        } else {
            insert
                .do_update()
                .set((
                    user_id.eq(excluded(user_id)),
                    app_version.eq(excluded(app_version)),
                    locale.eq(excluded(locale)),
                    last_seen_at.eq(now),
                    p256dh.eq(excluded(p256dh)),
                    auth.eq(excluded(auth)),
                ))
                .get_result(conn)
                .await?
        };
        Ok(device)
    }

    pub async fn find_for_user(uid: Uuid, conn: &mut DbConn<'_>) -> Result<Vec<Self>, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(push_devices
            .filter(user_id.eq(uid))
            .filter(revoked_at.is_null())
            .load(conn)
            .await?)
    }

    pub async fn is_active(device_id: Uuid, conn: &mut DbConn<'_>) -> Result<bool, DbError> {
        use crate::schema::push_devices::dsl::*;
        use diesel::OptionalExtension;
        let found: Option<Uuid> = push_devices
            .filter(id.eq(device_id))
            .filter(revoked_at.is_null())
            .select(id)
            .first(conn)
            .await
            .optional()?;
        Ok(found.is_some())
    }

    pub async fn is_active_for_user(
        device_id: Uuid,
        uid: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        use crate::schema::push_devices::dsl::*;
        use diesel::OptionalExtension;
        let found: Option<Uuid> = push_devices
            .filter(id.eq(device_id))
            .filter(user_id.eq(uid))
            .filter(revoked_at.is_null())
            .select(id)
            .first(conn)
            .await
            .optional()?;
        Ok(found.is_some())
    }

    pub async fn revoke_for_user(
        device_id: Uuid,
        uid: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(diesel::update(
            push_devices
                .filter(id.eq(device_id))
                .filter(user_id.eq(uid)),
        )
        .set(revoked_at.eq(Utc::now()))
        .execute(conn)
        .await?)
    }

    pub async fn revoke_by_token_for_user(
        uid: Uuid,
        token: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(diesel::update(
            push_devices
                .filter(user_id.eq(uid))
                .filter(device_token.eq(token)),
        )
        .set(revoked_at.eq(Utc::now()))
        .execute(conn)
        .await?)
    }

    pub async fn revoke_all_for_user(uid: Uuid, conn: &mut DbConn<'_>) -> Result<usize, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(diesel::update(push_devices.filter(user_id.eq(uid)))
            .set(revoked_at.eq(Utc::now()))
            .execute(conn)
            .await?)
    }

    pub async fn delete_by_token_for_user(
        uid: Uuid,
        token: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(diesel::delete(
            push_devices
                .filter(user_id.eq(uid))
                .filter(device_token.eq(token)),
        )
        .execute(conn)
        .await?)
    }

    pub async fn take_rotated_endpoint(
        uid: Uuid,
        old_token: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<bool, DbError> {
        use crate::schema::push_devices::dsl::*;
        let removed: Vec<Option<DateTime<Utc>>> = diesel::delete(
            push_devices
                .filter(user_id.eq(uid))
                .filter(device_token.eq(old_token)),
        )
        .returning(revoked_at)
        .get_results(conn)
        .await?;
        Ok(removed.iter().any(|r| r.is_some()))
    }

    pub async fn upsert_rotated(
        new: NewPushDevice,
        old_endpoint: Option<String>,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        let user = new.user_id;
        conn.transaction::<_, DbError, _>(move |tc| {
            async move {
                let carry_revocation = match &old_endpoint {
                    Some(old) if *old != new.device_token => {
                        Self::take_rotated_endpoint(user, old, tc).await?
                    }
                    _ => false,
                };
                let device = Self::upsert(new, false, tc).await?;
                if carry_revocation {
                    Self::revoke_for_user(device.id, user, tc).await?;
                }
                Ok(())
            }
            .scope_boxed()
        })
        .await
    }

    pub async fn touch(device_id: Uuid, conn: &mut DbConn<'_>) -> Result<usize, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(diesel::update(push_devices.filter(id.eq(device_id)))
            .set(last_seen_at.eq(Utc::now()))
            .execute(conn)
            .await?)
    }

    pub async fn delete_dead_token(
        plat: &str,
        token: &str,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(diesel::delete(
            push_devices
                .filter(platform.eq(plat))
                .filter(device_token.eq(token)),
        )
        .execute(conn)
        .await?)
    }

    pub async fn delete_stale(
        threshold: DateTime<Utc>,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(diesel::delete(
            push_devices
                .filter(last_seen_at.lt(threshold))
                .filter(revoked_at.is_null()),
        )
        .execute(conn)
        .await?)
    }
}
