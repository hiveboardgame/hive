use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{GameFinalize, HandlerOutput, InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, Schedule, Tournament},
    DbPool,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection};
use shared_types::{GameId, TournamentGameResult, TournamentId};
use uuid::Uuid;

pub struct AdjudicateResultHandler {
    user_id: Uuid,
    game_id: GameId,
    new_result: TournamentGameResult,
    pool: DbPool,
}

impl AdjudicateResultHandler {
    pub async fn new(
        game_id: GameId,
        new_result: TournamentGameResult,
        user_id: Uuid,
        pool: &DbPool,
    ) -> Result<Self> {
        Ok(Self {
            game_id,
            new_result,
            user_id,
            pool: pool.clone(),
        })
    }

    pub async fn handle(&self) -> Result<HandlerOutput> {
        let mut conn = get_conn(&self.pool).await?;
        let (tournament, finalized_game) = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move {
                    let game = Game::find_by_game_id(&self.game_id, tc).await?;
                    let game = game
                        .adjudicate_tournament_result(&self.user_id, &self.new_result, tc)
                        .await?;

                    if let Err(e) = Schedule::delete_all_for_game(game.id, tc).await {
                        println!("Failed to delete schedules for game {}: {}", game.id, e);
                    }

                    let id = game.tournament_id.expect("Have a tournament_id");
                    Ok((Tournament::find(id, tc).await?, game))
                }
                .scope_boxed()
            })
            .await?;

        let finalize_games = if finalized_game.finished {
            vec![GameFinalize {
                game_id: GameId(finalized_game.nanoid.clone()),
                white_id: finalized_game.white_id,
                black_id: finalized_game.black_id,
            }]
        } else {
            Vec::new()
        };
        let mut messages = Vec::new();
        for finalize in &finalize_games {
            messages.extend(finalize.own_game_removed_messages());
        }
        messages.push(InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Adjudicated(TournamentId(
                tournament.nanoid.clone(),
            ))),
        });

        Ok(HandlerOutput {
            messages,
            reactions: Vec::new(),
            finalize_games,
            subscriptions: Vec::new(),
        })
    }
}
