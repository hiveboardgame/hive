use crate::{
    common::{ServerMessage, TournamentUpdate},
    responses::TournamentResponse,
    websockets::{
        chat::Chats,
        internal_server_message::{InternalServerMessage, MessageDestination},
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use shared_types::TournamentId;
use uuid::Uuid;

pub struct GetHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    chat_storage: actix_web::web::Data<Chats>,
    pool: DbPool,
}

impl GetHandler {
    pub async fn new(
        tournament_id: TournamentId,
        user_id: Uuid,
        chat_storage: actix_web::web::Data<Chats>,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            tournament_id,
            user_id,
            chat_storage,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let tournament_response = TournamentResponse::from_model(&tournament, &mut conn).await?;
        let mut messages = Vec::new();
        let chat = self.chat_storage.tournament.read().unwrap();
        messages.push(InternalServerMessage {
            destination: MessageDestination::User(self.user_id),
            message: ServerMessage::Tournament(TournamentUpdate::Tournaments(vec![
                tournament_response,
            ])),
        });
        if let Some(messages_to_push) = chat.get(&self.tournament_id) {
            messages.push(InternalServerMessage {
                destination: MessageDestination::Tournament(self.tournament_id.clone()),
                message: ServerMessage::Chat(messages_to_push.clone()),
            });
        };
        Ok(messages)
    }
}
