use crate::schema::user_blocks;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = user_blocks)]
#[diesel(primary_key(blocker_id, blocked_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
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
