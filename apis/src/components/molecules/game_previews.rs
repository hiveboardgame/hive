use crate::{
    common::RatingChangeInfo,
    components::molecules::{
        rating_and_change::RatingAndChange, thumbnail_pieces::ThumbnailPieces, time_row::TimeRow,
    },
    responses::{GameResponse, UserResponse},
};
use hive_lib::{Color, GameStatus};
use leptos::prelude::*;
use shared_types::{Conclusion, GameSpeed, PrettyString, TimeInfo};

#[component]
pub fn GamePreviews(
    games: Callback<(), Vec<GameResponse>>,
    #[prop(optional)] show_time: bool,
) -> impl IntoView {
    let unfinished_ratings_view = move |wp: StoredValue<UserResponse>,
                                        bp: StoredValue<UserResponse>,
                                        base: Option<i32>,
                                        inc: Option<i32>| {
        let white = wp.get_value();
        let black = bp.get_value();
        view! {
            <div class="flex flex-wrap gap-1 justify-center p-1 text-center">
                {format!(
                    "{} {} vs {} {}",
                    white.username,
                    white
                        .ratings
                        .get(&GameSpeed::from_base_increment(base, inc))
                        .expect("White has a rating")
                        .rating,
                    black.username,
                    black
                        .ratings
                        .get(&GameSpeed::from_base_increment(base, inc))
                        .expect("Black has a rating")
                        .rating,
                )}

            </div>
        }
    };
    let finished_ratings_view =
        move |w_username: StoredValue<String>,
              b_username: StoredValue<String>,
              gs: StoredValue<GameStatus>,
              ratings: StoredValue<RatingChangeInfo>,
              conclusion: StoredValue<Conclusion>| {
            let game_result = match gs.get_value() {
                GameStatus::Finished(ref result) => result.to_string(),
                _ => "".to_string(),
            };
            view! {
                <div class="flex flex-wrap gap-1 justify-center p-1 w-full text-center">
                    <div class="flex flex-grow gap-1 items-center w-auto min-w-0 whitespace-nowrap max-w-[fit-content]">
                        <p>{w_username.get_value()}</p>
                        <RatingAndChange ratings side=Color::White />
                    </div>
                    <div class="w-auto text-center">vs</div>
                    <div class="flex flex-grow gap-1 items-center w-auto min-w-0 whitespace-nowrap max-w-[fit-content]">
                        <p>{b_username.get_value()}</p>
                        <RatingAndChange ratings side=Color::Black />
                    </div>

                </div>
                <div class="flex gap-1">
                    <div>{game_result.to_string()}</div>
                    {conclusion.get_value().pretty_string()}
                </div>
            }
        };
    view! {
        <div class="flex flex-row flex-wrap justify-center">
            <For each=move || games.run(()) key=|g| (g.game_id.clone(), g.turn) let:game>

                {
                    let board = game.create_state().board;
                    let base = game.time_base;
                    let inc = game.time_increment;
                    let finished = move || game.finished;
                    let rated = game.rated;
                    let game_id = game.game_id.clone();
                    let time_info = Signal::derive(move || TimeInfo {
                        mode: game.time_mode,
                        base,
                        increment: inc,
                    });
                    let ratings = StoredValue::new(RatingChangeInfo::from_game_response(&game));
                    let gs = StoredValue::new(game.game_status.clone());
                    let conclusion = StoredValue::new(game.conclusion.clone());
                    let w_username = StoredValue::new(game.white_player.username.clone());
                    let b_username = StoredValue::new(game.black_player.username.clone());
                    let white_player = StoredValue::new(game.white_player.clone());
                    let black_player = StoredValue::new(game.black_player.clone());
                    view! {
                        <div class="flex relative flex-col items-center m-2 w-60 h-60 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
                            <div class="flex flex-col items-center w-full">
                                <Show
                                    when=finished
                                    fallback=move || unfinished_ratings_view(
                                        white_player,
                                        black_player,
                                        base,
                                        inc,
                                    )
                                >

                                    {finished_ratings_view(
                                        w_username,
                                        b_username,
                                        gs,
                                        ratings,
                                        conclusion,
                                    )}

                                </Show>
                            </div>
                            <Show when=move || show_time>
                                <div class="flex items-center">
                                    {if rated { "RATED " } else { "CASUAL " }} <TimeRow time_info />
                                </div>
                            </Show>
                            <ThumbnailPieces board=StoredValue::new(board) />
                            <a
                                class="absolute top-0 left-0 z-10 w-full h-full"
                                href=format!("/game/{}", game_id)
                            ></a>
                        </div>
                    }
                }

            </For>
        </div>
    }
}
