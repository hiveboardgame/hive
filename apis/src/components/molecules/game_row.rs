use crate::{
    components::{
        atoms::{profile_link::ProfileLink, status_indicator::StatusIndicator, svgs::Svgs},
        molecules::{
            rating_and_change::RatingAndChange, thumbnail_pieces::ThumbnailPieces,
            time_row::TimeRow,
        },
    },
    responses::game::GameResponse,
};
use chrono::Utc;
use hive_lib::{color::Color, game_result::GameResult, game_status::GameStatus};
use leptos::*;
use leptos_icons::{Icon, RiIcon::RiSwordOthersLine};
use shared_types::time_mode::TimeMode;
use std::str::FromStr;

#[component]
pub fn GameRow(game: StoredValue<GameResponse>) -> impl IntoView {
    let rated_string = if game().rated { " RATED" } else { " CASUAL" };

    let result_string = match game().game_status {
        GameStatus::NotStarted | GameStatus::InProgress => "Playing now".to_string(),
        GameStatus::Finished(res) => match res {
            GameResult::Winner(c) => GameResult::Winner(c).to_string(),
            GameResult::Draw => GameResult::Draw.to_string(),
            _ => String::new(),
        },
    };

    let is_finished = move || (game().finished);
    let ago = move || {
        let (time, start_finish) = if game().finished {
            (
                Utc::now().signed_duration_since(game().updated_at),
                "Finished",
            )
        } else {
            (
                Utc::now().signed_duration_since(game().created_at),
                "Started",
            )
        };
        if time.num_weeks() > 1 {
            format!("{start_finish} {} weeks ago", time.num_weeks())
        } else if time.num_weeks() == 1 {
            format!("{start_finish} 1 week ago")
        } else if time.num_days() > 1 {
            format!("{start_finish} {} days ago", time.num_days())
        } else if time.num_days() == 1 {
            format!("{start_finish} 1 day ago")
        } else if time.num_hours() > 1 {
            format!("{start_finish} {} hours ago", time.num_hours())
        } else if time.num_hours() == 1 {
            format!("{start_finish} 1 hour ago")
        } else if time.num_minutes() > 1 {
            format!("{start_finish} {} minutes ago", time.num_minutes())
        } else if time.num_minutes() == 1 {
            format!("{start_finish} 1 minute ago")
        } else {
            format!("{start_finish} less than 1 minute ago")
        }
    };
    let history = game().history;
    let history_string = match history.len() {
        0 => String::from("No moves played"),
        _ => history
            .iter()
            .take(6)
            .enumerate()
            .map(|(i, (piece, dest))| format!("{}. {} {} ", i + 1, piece, dest))
            .chain(if history.len() > 6 {
                Some(String::from("⋯"))
            } else {
                None
            })
            .collect::<String>(),
    };

    view! {
        <article class="flex items-stretch h-72 px-2 py-4 duration-300 dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light relative mx-2 w-3/4 hover:bg-blue-light hover:dark:bg-blue-dark">
            <div class="h-60 w-60 mx-2">
                <svg
                    viewBox="1100 500 600 400"
                    class="touch-none h-full w-full"
                    xmlns="http://www.w3.org/2000/svg"
                >
                    <Svgs/>
                    <g>
                        <ThumbnailPieces game=game()/>
                    </g>
                </svg>
            </div>
            <div class="flex flex-col justify-between m-2 overflow-hidden w-full">
                <div class="flex flex-col justify-between">
                    <div class="flex gap-1">
                        {rated_string}
                        <TimeRow
                            time_mode=TimeMode::from_str(&game().time_mode)
                                .expect("Valid time mode")
                            time_base=game().time_base
                            increment=game().time_increment
                        />
                    </div>
                    <div>{ago}</div>

                </div>
                <div class="flex justify-center items-center w-full gap-1">
                    <div class="mr-2">
                        <div class="flex items-center">
                            <StatusIndicator username=game().white_player.username/>
                            <ProfileLink username=game().white_player.username/>
                        </div>
                        <br/>
                        <Show when=is_finished fallback=move || { game().white_player.rating }>
                            <RatingAndChange game=game() side=Color::White/>
                        </Show>

                    </div>
                    <Icon icon=Icon::from(RiSwordOthersLine)/>
                    <div class="ml-2">
                        <div class="flex items-center">
                            <StatusIndicator username=game().black_player.username/>
                            <ProfileLink username=game().black_player.username/>
                        </div>
                        <br/>
                        <Show when=is_finished fallback=move || { game().black_player.rating }>
                            <RatingAndChange game=game() side=Color::Black/>
                        </Show>
                    </div>
                </div>
                <div class="flex justify-center items-center w-full gap-1">{result_string}</div>
                {history_string}
            </div>
            <a
                class="h-full w-full absolute top-0 left-0 z-10"
                href=format!("/game/{}", game().nanoid)
            ></a>
        </article>
    }
}
