use crate::{
    common::{GameReaction, ServerMessage, TournamentUpdate},
    notifications::GameEndReason,
    websocket::{
        messages::{HandlerOutput, InternalServerMessage, MessageDestination, SocketTx},
        server_handlers::game::{
            append_fixed_field_commit,
            append_terminal_game_fanout,
            TerminalGameFanout,
        },
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, tournaments::fixed_field, DbPool};
use shared_types::TournamentId;
use std::sync::Arc;
use uuid::Uuid;

pub struct CloseoutHandler {
    tournament_id: TournamentId,
    slot_ids: Vec<Uuid>,
    actor_id: Uuid,
    actor_username: String,
    received_from: SocketTx,
    hub: Arc<WsHub>,
    pool: DbPool,
}

impl CloseoutHandler {
    pub fn new(
        tournament_id: TournamentId,
        slot_ids: Vec<Uuid>,
        actor_id: Uuid,
        actor_username: &str,
        received_from: SocketTx,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            slot_ids,
            actor_id,
            actor_username: actor_username.to_owned(),
            received_from,
            hub,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let outcome = fixed_field::close_unstarted_slots(
            tournament.id,
            self.actor_id,
            self.slot_ids.clone(),
            &mut conn,
        )
        .await?;

        let messages = vec![InternalServerMessage {
            destination: MessageDestination::Direct(self.received_from.clone()),
            message: ServerMessage::Tournament(TournamentUpdate::SlotsClosed {
                tournament_id: self.tournament_id.clone(),
                count: outcome.closed_slots,
            }),
        }];
        let mut output = HandlerOutput::from(messages);
        append_fixed_field_commit(outcome.commit, &mut output, &mut conn).await;

        let effects = outcome
            .terminal_games
            .iter()
            .map(|game| TerminalGameFanout {
                game,
                reaction: GameReaction::Adjudicated,
                actor_id: self.actor_id,
                actor_username: Some(&self.actor_username),
                end_reason: Some(GameEndReason::Agreement),
                notification_excluded: None,
                publish_tv: true,
                deleted_row: false,
            })
            .collect::<Vec<_>>();
        append_terminal_game_fanout(self.hub.data.as_ref(), &effects, &mut output, &mut conn).await;
        Ok(output)
    }
}
