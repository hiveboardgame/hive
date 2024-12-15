use crate::{
    providers::{
        games_search::ProfileGamesContext, navigation_controller::NavigationControllerSignal,
    },
    responses::GamesSearchResponse,
};
use leptos::prelude::*;
use shared_types::GamesContextToUpdate;

pub fn handle_games_search(results: GamesSearchResponse) {
    let ctx = expect_context::<ProfileGamesContext>();
    let navi = expect_context::<NavigationControllerSignal>();
    match results.ctx_to_update {
        GamesContextToUpdate::Profile(username) => {
            if navi
                .profile_signal
                .get()
                .username
                .is_some_and(|v| v == username)
            {
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
}
