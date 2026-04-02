use crate::schema::user_tournament_chat_mutes;
use chrono::{DateTime, Utc};
use diesel::{Insertable, Queryable, Selectable};
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug, Clone)]
#[diesel(table_name = user_tournament_chat_mutes)]
#[diesel(primary_key(user_id, tournament_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserTournamentChatMute {
    pub user_id: Uuid,
    pub tournament_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = user_tournament_chat_mutes)]
pub struct NewUserTournamentChatMute {
    pub user_id: Uuid,
    pub tournament_id: Uuid,
}
