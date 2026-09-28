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
use diesel_async::RunQueryDsl;
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
        explicit_registration: bool,
        conn: &mut DbConn<'_>,
    ) -> Result<Self, DbError> {
        use crate::schema::push_devices::dsl::*;
        let now = Utc::now();
        let device = if explicit_registration {
            diesel::insert_into(push_devices)
                .values(&new)
                .on_conflict((platform, device_token))
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
            // Background reconciliation cannot enroll a new device or change its owner.
            diesel::update(
                push_devices
                    .filter(user_id.eq(new.user_id))
                    .filter(platform.eq(&new.platform))
                    .filter(device_token.eq(&new.device_token)),
            )
            .set((
                app_version.eq(new.app_version),
                locale.eq(new.locale),
                last_seen_at.eq(now),
                p256dh.eq(new.p256dh),
                auth.eq(new.auth),
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

    pub async fn rotate_endpoint(
        new: NewPushDevice,
        old_endpoint: Option<String>,
        conn: &mut DbConn<'_>,
    ) -> Result<(), DbError> {
        use crate::schema::push_devices::dsl::*;
        let old_endpoint = old_endpoint.ok_or(DbError::Unauthorized)?;
        // One UPDATE preserves the owner and revocation atomically. The unique endpoint
        // constraint also prevents a renewal from replacing another registration.
        diesel::update(
            push_devices
                .filter(user_id.eq(new.user_id))
                .filter(platform.eq(new.platform))
                .filter(device_token.eq(old_endpoint)),
        )
        .set((
            device_token.eq(new.device_token),
            app_version.eq(new.app_version),
            locale.eq(new.locale),
            last_seen_at.eq(Utc::now()),
            p256dh.eq(new.p256dh),
            auth.eq(new.auth),
        ))
        .returning(id)
        .get_result::<Uuid>(conn)
        .await?;
        Ok(())
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
