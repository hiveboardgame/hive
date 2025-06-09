use crate::i18n::*;
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
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{GameStart, PrettyString, TimeInfo};

#[component]
pub fn GameRow(game: GameResponse) -> impl IntoView {
    let game = Signal::derive(move || game.clone());
    let i18n = use_i18n();
    let rated_string = move || match game().rated {
        true => t_string!(i18n, game.rated),
        false => t_string!(i18n, game.casual),
    };

    let result_string = move || match game().game_status {
        GameStatus::NotStarted => {
            if game().finished {
                game().tournament_game_result.to_string()
            } else if game().game_start == GameStart::Ready {
                t_string!(i18n, game.start_when.both_agree).to_string()
            } else {
                "Not started".to_string()
            }
        }
        GameStatus::InProgress => "Playing now".to_string(),
        GameStatus::Finished(res) => match res {
            GameResult::Winner(c) => GameResult::Winner(c).to_string(),
            GameResult::Draw => GameResult::Draw.to_string(),
            _ => game().conclusion.to_string(),
        },
    };

    let is_finished = move || (game().finished);
    let ago = move || match is_finished() {
        true => {
            let time = Utc::now().signed_duration_since(game().updated_at);
            if time.num_weeks() >= 1 {
                t_string!(i18n, game.finished_ago.weeks, count = time.num_weeks())
            } else if time.num_days() >= 1 {
                t_string!(i18n, game.finished_ago.days, count = time.num_days())
            } else if time.num_hours() >= 1 {
                t_string!(i18n, game.finished_ago.hours, count = time.num_hours())
            } else if time.num_minutes() >= 1 {
                t_string!(i18n, game.finished_ago.minutes, count = time.num_minutes())
            } else {
                t_string!(i18n, game.finished_ago.less_than_minute).to_string()
            }
        }
        false => {
            let time = Utc::now().signed_duration_since(game().created_at);
            if time.num_weeks() >= 1 {
                t_string!(i18n, game.created_ago.weeks, count = time.num_weeks())
            } else if time.num_days() >= 1 {
                t_string!(i18n, game.created_ago.days, count = time.num_days())
            } else if time.num_hours() >= 1 {
                t_string!(i18n, game.created_ago.hours, count = time.num_hours())
            } else if time.num_minutes() >= 1 {
                t_string!(i18n, game.created_ago.minutes, count = time.num_minutes())
            } else {
                t_string!(i18n, game.created_ago.less_than_minute).to_string()
            }
        }
    };
    let history = game().history;
    let history_string = move || match history.len() {
        0 => t_string!(i18n, game.no_moves_played).to_string(),
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
    let ratings = StoredValue::new(RatingChangeInfo::from_game_response(&game()));
    let time_info = TimeInfo {
        mode: game().time_mode,
        base: game().time_base,
        increment: game().time_increment,
    };
    let board = game().create_state().board;
    view! {
        <article class="grid relative flex-col px-2 py-4 mx-2 w-full h-40 duration-300 sm:h-56 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
            <div class="flex flex-col gap-1">
                <p class="flex gap-1 justify-center truncate">
                    {rated_string} <TimeRow time_info /> {ago}
                </p>
                <Show when=move || { game().tournament.is_some() }>
                    <p class="flex gap-1 justify-center truncate">
                        {t!(i18n, game.played_in)}
                        <a
                            class="z-20 text-blue-500 truncate hover:underline"
                            href=format!("/tournament/{}", game().tournament.unwrap().tournament_id)
                        >
                            {game().tournament.unwrap().name}
                        </a>
                    </p>
                </Show>
            </div>
            <div class="flex absolute inset-y-0 left-0 w-1/2">
                <ThumbnailPieces board=StoredValue::new(board) />
            </div>
            <div class="grid flex-col justify-between justify-self-end w-1/2">
                <Show when=is_finished>
                    <p class="text-center">{format!("{} {}", conclusion(), result_string())}</p>
                </Show>
                <div class="flex gap-1 justify-center items-center w-full">
                    <div class="mr-2">
                        <div class="flex items-center">
                            <StatusIndicator username=game().white_player.username />
                            <ProfileLink
                                patreon=game().white_player.patreon
                                username=game().white_player.username
                            />
                        </div>
                        <Show when=is_finished fallback=move || { game().white_rating() }>
                            <div class="flex gap-1">
                                <RatingAndChange ratings side=Color::White />
                            </div>
                        </Show>

                    </div>
                    <Icon icon=icondata::RiSwordOthersLine />
                    <div class="ml-2">
                        <div class="flex items-center">
                            <StatusIndicator username=game().black_player.username />
                            <ProfileLink
                                username=game().black_player.username
                                patreon=game().black_player.patreon
                            />
                        </div>
                        <Show when=is_finished fallback=move || { game().black_rating() }>
                            <div class="flex gap-1">
                                <RatingAndChange ratings side=Color::Black />
                            </div>
                        </Show>
                    </div>
                </div>
                <p class="text-center truncate whitespace-pre-line">{history_string}</p>
            </div>
            <a
                class="absolute top-0 left-0 w-full h-full z-25"
                href=format!("/game/{}", game().game_id)
            ></a>
            <div class="absolute right-0 bottom-0 m-2">
                <DownloadPgn game=StoredValue::new(game()) />
            </div>
        </article>
    }
}
