use crate::schema::user_blocks;
use diesel::Insertable;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = user_blocks)]
pub(crate) struct NewUserBlock {
    pub blocker_id: Uuid,
    pub blocked_id: Uuid,
}
