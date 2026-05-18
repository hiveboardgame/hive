use crate::{db_error::DbError, schema::push_devices, DbConn};
use chrono::{DateTime, Utc};
use diesel::{
    upsert::excluded, ExpressionMethods, Identifiable, Insertable, QueryDsl, Queryable, Selectable,
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
}

impl PushDevice {
    /// Register-or-update by (platform, device_token). Re-registering the
    /// same token rebinds it to the current user and refreshes metadata —
    /// covers token rotation and "different account on same phone".
    pub async fn upsert(new: NewPushDevice, conn: &mut DbConn<'_>) -> Result<Self, DbError> {
        use crate::schema::push_devices::dsl::*;
        let now = Utc::now();
        Ok(diesel::insert_into(push_devices)
            .values(&new)
            .on_conflict((platform, device_token))
            .do_update()
            .set((
                user_id.eq(excluded(user_id)),
                app_version.eq(excluded(app_version)),
                locale.eq(excluded(locale)),
                last_seen_at.eq(now),
            ))
            .get_result(conn)
            .await?)
    }

    pub async fn find_for_user(uid: Uuid, conn: &mut DbConn<'_>) -> Result<Vec<Self>, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(push_devices.filter(user_id.eq(uid)).load(conn).await?)
    }

    pub async fn delete_for_user(
        device_id: Uuid,
        uid: Uuid,
        conn: &mut DbConn<'_>,
    ) -> Result<usize, DbError> {
        use crate::schema::push_devices::dsl::*;
        Ok(diesel::delete(
            push_devices
                .filter(id.eq(device_id))
                .filter(user_id.eq(uid)),
        )
        .execute(conn)
        .await?)
    }

    /// Drop a token APNs/FCM reported as unregistered.
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
}
