use crate::{providers::games_search::ProfileGamesContext, responses::GamesSearchResponse};
use leptos::*;
use shared_types::GamesContextToUpdate;

pub fn handle_games_search(results: GamesSearchResponse) {
    let ctx = expect_context::<ProfileGamesContext>();
    match results.ctx_to_update {
        GamesContextToUpdate::Profile => {
            ctx.games.update(|games| {
                if results.first_batch {
                    *games = results.results;
                } else {
                    games.extend(results.results);
                }
            });
            ctx.batch_info.update(|batch| {
                *batch = results.batch;
            });
            ctx.more_games.update(|more_finished| {
                *more_finished = results.more_rows;
            });
        }
    }
}
