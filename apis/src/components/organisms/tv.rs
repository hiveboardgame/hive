use crate::{
    components::molecules::{thumbnail_pieces::ThumbnailPieces, time_row::TimeRow},
    providers::games::GamesSignal,
};
use leptos::*;
use shared_types::GameSpeed;

#[component]
pub fn Tv() -> impl IntoView {
    let games = expect_context::<GamesSignal>();
    let live_games = move || (games.live)().live_games;

    view! {
        <div class="flex flex-col items-center pt-6 lg:w-[780px] md:w-[700px]">
            <div class="flex flex-col flex-wrap gap-1 justify-center items-center w-full md:flex-row">
                <For each=live_games key=|(k, v)| (k.to_owned(), v.turn) let:game>
                    <div class="flex relative flex-col items-center mx-2 w-60 h-60 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
                        <div class="flex flex-col items-center">
                            {format!(
                                "{} {} vs {} {}",
                                game.1.white_player.username,
                                game
                                    .1
                                    .white_player
                                    .ratings
                                    .get(
                                        &GameSpeed::from_base_increment(
                                            game.1.time_base,
                                            game.1.time_increment,
                                        ),
                                    )
                                    .unwrap()
                                    .rating,
                                game.1.black_player.username,
                                game
                                    .1
                                    .black_player
                                    .ratings
                                    .get(
                                        &GameSpeed::from_base_increment(
                                            game.1.time_base,
                                            game.1.time_increment,
                                        ),
                                    )
                                    .unwrap()
                                    .rating,
                            )}
                            <div class="flex items-center">
                                {if game.1.rated { "RATED " } else { "CASUAL " }}
                                <TimeRow
                                    time_mode=game.1.time_mode.clone()
                                    time_base=game.1.time_base
                                    increment=game.1.time_increment
                                />
                            </div>
                        </div>
                        <ThumbnailPieces game=game.1/>
                        <a
                            class="absolute top-0 left-0 z-10 w-full h-full"
                            href=format!("/game/{}", game.0)
                        ></a>
                    </div>
                </For>
            </div>
        </div>
    }
}
