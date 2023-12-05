use crate::{
    components::{atoms::svgs::Svgs, molecules::thumbnail_pieces::ThumbnailPieces},
    functions::games::game_response::GameStateResponse,
};
use hive_lib::{game_result::GameResult, game_status::GameStatus};
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
    let white_href = format!("/@/{}", game().white_player.username);
    let black_href = format!("/@/{}", game().black_player.username);

    view! {
        <article class="hover:bg-blue-300 flex items-stretch h-72 px-2 py-4 border border-gray-800 dark:border-gray-400 relative mx-2 w-3/4">
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
                        <a class="z-20 relative font-bold hover:text-blue-600" href=white_href>

                            {game().white_player.username}
                        </a>
                        <br/>
                        {game().white_player.rating}
                    </div>
                    <Icon icon=Icon::from(RiSwordOthersLine) class=""/>
                    <div>
                        <a class="z-20 relative font-bold hover:text-blue-600" href=black_href>

                            {game().black_player.username}
                        </a>
                        <br/>
                        {game().black_player.rating}
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
