use crate::schema::user_tournament_chat_mutes;
use diesel::Insertable;
use uuid::Uuid;

#[derive(Insertable, Debug)]
#[diesel(table_name = user_tournament_chat_mutes)]
pub(crate) struct NewUserTournamentChatMute {
    pub user_id: Uuid,
    pub tournament_id: Uuid,
}
