use crate::{
    common::RatingChangeInfo,
    components::{
        atoms::{
            download_pgn::DownloadPgn, profile_link::ProfileLink, status_indicator::StatusIndicator,
        },
        molecules::{
            rating_and_change::RatingAndChange, thumbnail_pieces::ThumbnailPieces,
            time_row::TimeRow,
        },
    },
    responses::GameResponse,
};
use chrono::Utc;
use hive_lib::{Color, GameResult, GameStatus};
use leptos::*;
use leptos_icons::*;
use shared_types::{GameStart, PrettyString, TimeInfo};

#[component]
pub fn GameRow(game: StoredValue<GameResponse>) -> impl IntoView {
    let rated_string = if game().rated { " RATED" } else { " CASUAL" };

    let result_string = match game().game_status {
        GameStatus::NotStarted => {
            if game().game_start == GameStart::Ready {
                "The game will start once both players agree on a time".to_string()
            } else {
                "Not started".to_string()
            }
        }
        GameStatus::InProgress => "Playing now".to_string(),
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
                "Created",
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
    let conclusion = move || game().conclusion.pretty_string();
    let ratings = store_value(RatingChangeInfo::from_game_response(&game()));
    let time_info = TimeInfo {
        mode: game().time_mode.clone(),
        base: game().time_base,
        increment: game().time_increment,
    };

    view! {
        <article class="flex relative px-2 py-4 mx-2 w-full h-72 duration-300 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
            <div class="mx-2 w-60 h-60">
                <ThumbnailPieces game=game()/>
            </div>
            <div class="flex overflow-hidden flex-col justify-between m-2 w-full">
                <div class="flex flex-col justify-between">
                    <div class="flex gap-1">
                        {rated_string} <TimeRow time_info/>
                        <Show when=move || {
                            game().tournament.is_some()
                        }>
                            played in
                            <a
                                class="z-20 text-blue-500 hover:underline"
                                href=format!(
                                    "/tournament/{}",
                                    game().tournament.unwrap().tournament_id,
                                )
                            >

                                {game().tournament.unwrap().name}
                            </a>
                        </Show>
                    </div>
                    <div>{ago}</div>
                </div>
                <div class="flex gap-1 justify-center items-center w-full">
                    <div class="mr-2">
                        <div class="flex items-center">
                            <StatusIndicator username=game().white_player.username/>
                            <ProfileLink
                                patreon=game().white_player.patreon
                                username=game().white_player.username
                            />
                        </div>
                        <br/>
                        <Show when=is_finished fallback=move || { game().white_rating() }>
                            <RatingAndChange ratings=ratings() side=Color::White/>
                        </Show>

                    </div>
                    <Icon icon=icondata::RiSwordOthersLine/>
                    <div class="ml-2">
                        <div class="flex items-center">
                            <StatusIndicator username=game().black_player.username/>
                            <ProfileLink
                                username=game().black_player.username
                                patreon=game().black_player.patreon
                            />
                        </div>
                        <br/>
                        <Show when=is_finished fallback=move || { game().black_rating() }>
                            <RatingAndChange ratings=ratings() side=Color::Black/>
                        </Show>
                    </div>
                </div>
                <div class="flex gap-1 justify-center items-center w-full">
                    {result_string} <Show when=is_finished>{conclusion}</Show>
                </div>
                <div class="flex gap-1 justify-between items-center w-full">
                    {history_string} <DownloadPgn game=game/>
                </div>
            </div>
            <a
                class="absolute top-0 left-0 z-10 w-full h-full"
                href=format!("/game/{}", game().game_id)
            ></a>
        </article>
    }
}
