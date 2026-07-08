use crate::{db_error::DbError, models::User, schema::user_blocks, DbConn};
use chrono::{DateTime, Utc};
use diesel::{dsl::exists, prelude::*, select, Insertable, Queryable, Selectable};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = user_blocks)]
#[diesel(primary_key(blocker_id, blocked_id))]
pub struct UserBlock {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = user_blocks)]
pub struct NewUserBlock {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
}

impl UserBlock {
    pub async fn block(
        conn: &mut DbConn<'_>,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<(), DbError> {
        if blocker_id == blocked_id {
            return Err(DbError::InvalidInput {
                info: "Cannot block yourself".to_string(),
                error: "blocker_id == blocked_id".to_string(),
            });
        }
        User::find_active_by_uuid(&blocked_id, conn).await?;

        diesel::insert_into(user_blocks::table)
            .values(NewUserBlock {
                blocker_id,
                blocked_id,
            })
            .on_conflict((user_blocks::blocker_id, user_blocks::blocked_id))
            .do_nothing()
            .execute(conn)
            .await
            .map_err(DbError::from)?;
        Ok(())
    }

    pub async fn unblock(
        conn: &mut DbConn<'_>,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<(), DbError> {
        diesel::delete(
            user_blocks::table
                .filter(user_blocks::blocker_id.eq(blocker_id))
                .filter(user_blocks::blocked_id.eq(blocked_id)),
        )
        .execute(conn)
        .await
        .map_err(DbError::from)?;
        Ok(())
    }

    pub async fn is_blocked(
        conn: &mut DbConn<'_>,
        blocker_id: Uuid,
        blocked_id: Uuid,
    ) -> Result<bool, DbError> {
        select(exists(
            user_blocks::table
                .filter(user_blocks::blocker_id.eq(blocker_id))
                .filter(user_blocks::blocked_id.eq(blocked_id)),
        ))
        .get_result(conn)
        .await
        .map_err(DbError::from)
    }

    pub async fn blocked_user_ids(
        conn: &mut DbConn<'_>,
        blocker_id: Uuid,
    ) -> Result<Vec<Uuid>, DbError> {
        user_blocks::table
            .filter(user_blocks::blocker_id.eq(blocker_id))
            .select(user_blocks::blocked_id)
            .load(conn)
            .await
            .map_err(DbError::from)
    }
}
