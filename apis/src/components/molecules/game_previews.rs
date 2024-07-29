use crate::{
    common::RatingChangeInfo,
    components::molecules::{
        rating_and_change::RatingAndChange, thumbnail_pieces::ThumbnailPieces, time_row::TimeRow,
    },
    responses::GameResponse,
};
use hive_lib::{Color, GameStatus};
use leptos::*;
use shared_types::{GameSpeed, PrettyString, TimeInfo};

#[component]
pub fn GamePreviews(
    games: Callback<(), Vec<GameResponse>>,
    #[prop(optional)] show_time: bool,
) -> impl IntoView {
    let usernames_with_rating = move |game: GameResponse| {
        let ratings = store_value(RatingChangeInfo::from_game_response(&game));

        if game.finished {
            let game_result = match game.game_status {
                GameStatus::Finished(ref result) => result.to_string(),
                _ => "".to_string(),
            };

            view! {
                <div class="flex gap-1">
                    {game.white_player.username}
                    <RatingAndChange ratings=ratings() side=Color::White/> vs
                    {game.black_player.username}
                    <RatingAndChange ratings=ratings() side=Color::Black/>
                </div>
                <div class="flex gap-1">
                    <div>{game_result}</div>
                    {game.conclusion.pretty_string()}
                </div>
            }
            .into_view()
        } else {
            view! {
                {format!(
                    "{} {} vs {} {}",
                    game.white_player.username,
                    game
                        .white_player
                        .ratings
                        .get(&GameSpeed::from_base_increment(game.time_base, game.time_increment))
                        .expect("White has a rating")
                        .rating,
                    game.black_player.username,
                    game
                        .black_player
                        .ratings
                        .get(&GameSpeed::from_base_increment(game.time_base, game.time_increment))
                        .expect("Black has a rating")
                        .rating,
                )}
            }
            .into_view()
        }
    };
    view! {
        <For each=move || games(()) key=|g| (g.game_id.clone(), g.turn) let:game>

            {
                let time_info = store_value(TimeInfo {
                    mode: game.time_mode,
                    base: game.time_base,
                    increment: game.time_increment,
                });
                let game = store_value(game);
                let needs_start = move || {
                    let game = game();
                    game.tournament.is_some() && matches!(game.game_status, GameStatus::NotStarted)
                };
                view! {
                    <div class="flex relative flex-col items-center m-2 w-60 h-60 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
                        <div class="flex flex-col items-center">
                            {usernames_with_rating(game())}
                        </div>
                        <Show when=move || show_time>
                            <div class="flex items-center">
                                {if game().rated { "RATED " } else { "CASUAL " }}
                                <TimeRow time_info=time_info().into()/>
                            </div>
                        </Show>
                        <Show when=needs_start>
                            <div class="flex p-2">
                                "The game will start once both players agree on a time"
                            </div>
                        </Show>
                        <ThumbnailPieces game=game()/>
                        <a
                            class="absolute top-0 left-0 z-10 w-full h-full"
                            href=format!("/game/{}", game().game_id)
                        ></a>
                    </div>
                }
            }

        </For>
    }
}
