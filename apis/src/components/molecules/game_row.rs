use crate::{
    components::{
        atoms::{profile_link::ProfileLink, svgs::Svgs},
        molecules::{rating_and_change::RatingAndChange, thumbnail_pieces::ThumbnailPieces},
    },
    functions::games::game_response::GameStateResponse,
};
use hive_lib::{color::Color, game_result::GameResult, game_status::GameStatus};
use leptos::*;
use leptos_icons::{Icon, RiIcon::RiSwordOthersLine};

#[component]
pub fn GameRow(game: StoredValue<GameStateResponse>) -> impl IntoView {
    let rated_string = if game().rated { " RATED" } else { " CASUAL" };

    let result_string = match game().game_status {
        GameStatus::NotStarted | GameStatus::InProgress => "Playing now".to_string(),
        GameStatus::Finished(res) => match res {
            GameResult::Winner(c) => GameResult::Winner(c).to_string(),
            GameResult::Draw => GameResult::Draw.to_string(),
            _ => String::new(),
        },
    };

    let is_finished = move || match game().game_status {
        GameStatus::Finished(_) => true,
        _ => false,
    };

    view! {
        <article class="flex items-stretch h-72 px-2 py-4 dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light relative mx-2 w-3/4 hover:bg-blue-light hover:dark:bg-blue-dark">
            <div class="h-60 w-60 mx-2">
                <svg
                    viewBox="1100 500 600 400"
                    class="touch-none h-full w-full"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <Svgs/>
                    <g class="h-full w-full">
                        <ThumbnailPieces game=game()/>
                    </g>
                </svg>
            </div>
            <div class="flex flex-col justify-between m-2 overflow-hidden w-full">
                <div class="flex flex-col justify-between">
                    <strong>"ICON AND TIMECONTROL? " {rated_string}</strong>
                    <time>Game Started at time</time>
                </div>
                <div class="flex justify-center items-center w-full gap-1">
                    <div>
                        <ProfileLink username=game().white_player.username/>
                        <br/>
                        <Show when=is_finished fallback=move || { game().white_player.rating }>
                            <RatingAndChange game=game() side=Color::White/>
                        </Show>

                    </div>
                    <Icon icon=Icon::from(RiSwordOthersLine) class=""/>
                    <div>
                        <ProfileLink username=game().black_player.username/>
                        <br/>
                        <Show when=is_finished fallback=move || { game().black_player.rating }>
                            <RatingAndChange game=game() side=Color::Black/>
                        </Show>
                    </div>
                </div>
                <div class="flex justify-center items-center w-full gap-1">{result_string}</div>
                <div>Turn: {game().turn}</div>
            </div>
            <a
                class="h-full w-full absolute top-0 left-0 z-10"
                href=format!("/game/{}", game().nanoid)
            ></a>
        </article>
    }
}
