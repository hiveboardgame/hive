use crate::{
    common::{ServerMessage, TournamentUpdate},
    websocket::messages::{InternalServerMessage, MessageDestination},
};
use anyhow::Result;
use db_lib::{
    get_conn,
    models::{Game, Tournament},
    DbPool,
};
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::AsyncConnection;
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

    pub async fn handle(&self) -> Result<Vec<InternalServerMessage>> {
        let mut conn = get_conn(&self.pool).await?;
        let tournament = conn
            .transaction::<_, anyhow::Error, _>(move |tc| {
                async move {
                    let game = Game::find_by_game_id(&self.game_id, tc).await?;
                    game.adjudicate_tournament_result(&self.user_id, &self.new_result, tc)
                        .await?;
                    let id = game.tournament_id.expect("Have a tournament_id");
                    Ok(Tournament::find(id, tc).await?)
                }
                .scope_boxed()
            })
            .await?;

        Ok(vec![InternalServerMessage {
            destination: MessageDestination::Global,
            message: ServerMessage::Tournament(TournamentUpdate::Adjudicated(TournamentId(
                tournament.nanoid.clone(),
            ))),
        }])
    }
}
