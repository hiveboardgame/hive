use crate::{
    common::{GameReaction, TournamentAdjudicationIntent},
    websocket::{
        messages::HandlerOutput,
        server_handlers::game::{
            append_fixed_field_commit,
            project_committed_game,
            GameCommandContext,
            GameProjectionInput,
        },
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::Tournament,
    tournaments::fixed_field::{self, AdjudicationOutcome},
    DbPool,
};
use shared_types::{GameId, TournamentId};
use std::sync::Arc;
use uuid::Uuid;

pub struct AdjudicateResultHandler {
    tournament_id: TournamentId,
    intent: TournamentAdjudicationIntent,
    user_id: Uuid,
    username: String,
    hub: Arc<WsHub>,
    pool: DbPool,
}

impl AdjudicateResultHandler {
    pub fn new(
        tournament_id: TournamentId,
        intent: TournamentAdjudicationIntent,
        user_id: Uuid,
        username: &str,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            intent,
            user_id,
            username: username.to_owned(),
            hub,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let slot_id = self.intent.slot_id;
        let outcome = fixed_field::adjudicate_slot_atomic(
            tournament.id,
            slot_id,
            self.intent.result.clone(),
            self.intent.expected_resolution,
            self.user_id,
            &mut conn,
        )
        .await?;
        let AdjudicationOutcome {
            cleared,
            newly_terminal,
            committed_game,
            commit,
        } = outcome;
        let Some(game) = committed_game else {
            let mut output = HandlerOutput::empty();
            append_fixed_field_commit(commit, &mut output, &mut conn).await;
            return Ok(output);
        };
        if cleared {
            self.hub
                .restore_game_heartbeat_for_viewers(&GameId(game.nanoid.clone()));
        }
        let reaction = if cleared {
            GameReaction::Reopened
        } else {
            GameReaction::Adjudicated
        };
        let context = GameCommandContext::websocket(reaction, self.user_id, self.username.clone());
        let projected = project_committed_game(
            GameProjectionInput::Committed {
                game,
                newly_terminal,
            },
            context,
            self.hub.data.as_ref(),
            &mut conn,
        )
        .await;
        let mut output = projected.output;
        append_fixed_field_commit(commit, &mut output, &mut conn).await;
        Ok(output)
    }
}
