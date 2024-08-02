use crate::{pages::profile_view::ProfileGamesContext, responses::GamesSearchResponse};
use hive_lib::GameStatus;
use leptos::*;
use shared_types::{GameStart, GamesContextToUpdate};

pub fn handle_games_search(results: GamesSearchResponse) {
    let ctx = expect_context::<ProfileGamesContext>();
    match results.ctx_to_update {
        GamesContextToUpdate::ProfileFinished => {
            ctx.finished.update(|games| {
                games.extend(results.results);
            });
            ctx.finished_last_id.update(|last_id| {
                *last_id = results.last_id;
            });
            ctx.finished_last_timestamp.update(|last_timestamp| {
                *last_timestamp = results.last_timestamp;
            });
            ctx.more_finished.update(|more_finished| {
                *more_finished = results.more_rows;
            });
        }
        GamesContextToUpdate::ProfilePlaying => {
            ctx.playing.set_untracked(results.results);
            let mut unstarted = Vec::new();
            ctx.playing.update(|playing| {
                playing.retain(|gr| {
                    if gr.game_start == GameStart::Ready && gr.game_status == GameStatus::NotStarted
                    {
                        unstarted.push(gr.clone());
                        false
                    } else {
                        true
                    }
                })
            });
            ctx.unstarted.set(unstarted);
        }
    }
}
