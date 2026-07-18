use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{GameFinalize, HandlerOutput, InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{get_conn, models::Tournament, DbPool};
use diesel_async::AsyncConnection;
use hive_lib::GameStatus;
use shared_types::{GameId, TournamentId};
use uuid::Uuid;

pub enum BulkAdjudication {
    DoubleForfeitUnstarted,
    ResetAdjudicated,
}

pub struct BulkAdjudicateHandler {
    tournament_id: TournamentId,
    user_id: Uuid,
    pool: DbPool,
    action: BulkAdjudication,
}

impl BulkAdjudicateHandler {
    pub fn new(
        tournament_id: TournamentId,
        user_id: Uuid,
        action: BulkAdjudication,
        pool: &DbPool,
    ) -> Self {
        Self {
            tournament_id,
            user_id,
            pool: pool.clone(),
            action,
        }
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = Tournament::find_by_tournament_id(&self.tournament_id, &mut conn).await?;

        let finalized_games = conn
            .transaction::<_, anyhow::Error, _>(async move |tc| {
                let tournament = tournament.clone();
                let user_id = self.user_id;
                let action = &self.action;
                match action {
                    BulkAdjudication::DoubleForfeitUnstarted => {
                        let finalized_games = tournament
                            .games(tc)
                            .await?
                            .into_iter()
                            .filter(|game| game.game_status == GameStatus::NotStarted.to_string())
                            .collect();
                        tournament
                            .double_forfeit_unstarted_games(&user_id, tc)
                            .await?;
                        Ok(finalized_games)
                    }
                    BulkAdjudication::ResetAdjudicated => {
                        tournament.reset_adjudicated_games(&user_id, tc).await?;
                        Ok(Vec::new())
                    }
                }
            })
            .await?;

        let finalize_games: Vec<GameFinalize> = finalized_games
            .into_iter()
            .map(|game| GameFinalize {
                game_id: GameId(game.nanoid),
                white_id: game.white_id,
                black_id: game.black_id,
            })
            .collect();
        let mut messages = Vec::new();
        for finalize in &finalize_games {
            messages.extend(finalize.own_game_removed_messages());
        }
        messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Adjudicated(
                self.tournament_id.clone(),
            )),
        });

        Ok(HandlerOutput {
            messages,
            reactions: Vec::new(),
            finalize_games,
        })
    }
}
