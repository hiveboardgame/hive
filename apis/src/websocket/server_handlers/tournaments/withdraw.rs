use crate::{
    common::GameReaction,
    notifications::{game_end_reason_from, GameEndReason},
    websocket::{
        messages::HandlerOutput,
        server_handlers::{
            game::{append_fixed_field_commit, append_terminal_game_fanout, TerminalGameFanout},
            tournaments::{append_public_tournament_patches, PublicTournamentSection},
        },
        WsHub,
    },
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, tournaments::fixed_field, DbPool};
use shared_types::{Conclusion, TournamentId};
use std::sync::Arc;
use uuid::Uuid;

pub struct WithdrawHandler {
    tournament_id: TournamentId,
    player: Uuid,
    actor: Uuid,
    hub: Arc<WsHub>,
    pool: DbPool,
}

impl WithdrawHandler {
    pub fn new(
        tournament_id: TournamentId,
        player: Uuid,
        actor: Uuid,
        hub: Arc<WsHub>,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            player,
            actor,
            hub,
            pool: pool.clone(),
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;
        let outcome =
            fixed_field::withdraw_player(tournament.id, self.player, self.actor, &mut conn).await?;

        let mut output = HandlerOutput::empty();
        if outcome.applied {
            append_public_tournament_patches(
                tournament.id,
                &[PublicTournamentSection::Memberships],
                &mut output,
                &mut conn,
            )
            .await;
        }
        append_fixed_field_commit(outcome.commit, &mut output, &mut conn).await;

        let mut effects = Vec::with_capacity(outcome.terminal_games.len());
        for game in &outcome.terminal_games {
            let timed_out = game.conclusion == Conclusion::Timeout.to_string();
            let actor_id = if timed_out {
                game.current_player_id
            } else {
                self.player
            };
            effects.push(TerminalGameFanout {
                game,
                reaction: if timed_out {
                    GameReaction::TimedOut
                } else {
                    match game.last_game_control() {
                        Ok(Some(control)) => GameReaction::Control(control),
                        Ok(None) => GameReaction::Adjudicated,
                        Err(error) => {
                            log::error!(
                                "Tournament withdrawal committed Game {}, but its control history could not be projected: {error}",
                                game.nanoid,
                            );
                            GameReaction::Adjudicated
                        }
                    }
                },
                actor_id,
                actor_username: None,
                end_reason: Some(game_end_reason_from(game, GameEndReason::Resignation)),
                notification_excluded: None,
                publish_tv: true,
                deleted_row: false,
            });
        }
        append_terminal_game_fanout(self.hub.data.as_ref(), &effects, &mut output, &mut conn).await;
        Ok(output)
    }
}
