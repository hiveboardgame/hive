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
use leptos::*;
use leptos_icons::*;
use shared_types::{GameStart, PrettyString, TimeInfo};

#[component]
pub fn GameRow(game: StoredValue<GameResponse>) -> impl IntoView {
    let i18n = use_i18n();
    let rated_string = if game().rated {
        t!(i18n, game.rated).into_view()
    } else {
        t!(i18n, game.casual).into_view()
    };

    let result_string = match game().game_status {
        GameStatus::NotStarted => {
            if game().game_start == GameStart::Ready {
                t!(i18n, game.start_when.both_agree).into_view()
            } else {
                "Not started".into_view()
            }
        }
        GameStatus::InProgress => "Playing now".into_view(),
        GameStatus::Finished(res) => match res {
            GameResult::Winner(c) => GameResult::Winner(c).to_string().into_view(),
            GameResult::Draw => GameResult::Draw.to_string().into_view(),
            _ => "".into_view(),
        },
    };

    let is_finished = move || (game().finished);
    let ago = move || {
        if game().finished {
            let time = Utc::now().signed_duration_since(game().updated_at);
            if time.num_weeks() >= 1 {
                t!(
                    i18n,
                    game.finished_ago.weeks,
                    count = move || time.num_weeks()
                )
                .into_view()
            } else if time.num_days() >= 1 {
                t!(
                    i18n,
                    game.finished_ago.days,
                    count = move || time.num_days()
                )
                .into_view()
            } else if time.num_hours() >= 1 {
                t!(
                    i18n,
                    game.finished_ago.hours,
                    count = move || time.num_hours()
                )
                .into_view()
            } else if time.num_minutes() >= 1 {
                t!(
                    i18n,
                    game.finished_ago.minutes,
                    count = move || time.num_minutes()
                )
                .into_view()
            } else {
                t!(i18n, game.finished_ago.less_than_minute).into_view()
            }
        } else {
            let time = Utc::now().signed_duration_since(game().created_at);
            if time.num_weeks() >= 1 {
                t!(
                    i18n,
                    game.created_ago.weeks,
                    count = move || time.num_weeks()
                )
                .into_view()
            } else if time.num_days() >= 1 {
                t!(i18n, game.created_ago.days, count = move || time.num_days()).into_view()
            } else if time.num_hours() >= 1 {
                t!(
                    i18n,
                    game.created_ago.hours,
                    count = move || time.num_hours()
                )
                .into_view()
            } else if time.num_minutes() >= 1 {
                t!(
                    i18n,
                    game.created_ago.minutes,
                    count = move || time.num_minutes()
                )
                .into_view()
            } else {
                t!(i18n, game.created_ago.less_than_minute).into_view()
            }
        }
    };
    let history = game().history;
    let history_string = match history.len() {
        0 => t!(i18n, game.no_moves_played).into_view(),
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
            .collect::<String>()
            .into_view(),
    };
    let conclusion = move || game().conclusion.pretty_string();
    let ratings = store_value(RatingChangeInfo::from_game_response(&game()));
    let time_info = TimeInfo {
        mode: game().time_mode,
        base: game().time_base,
        increment: game().time_increment,
    };

    view! {
        <article class="flex relative px-2 py-4 mx-2 w-full h-72 duration-300 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
            <div class="mx-2 w-60 h-60">
                <ThumbnailPieces game />
            </div>
            <div class="flex overflow-hidden flex-col justify-between m-2 w-full">
                <div class="flex flex-col justify-between">
                    <div class="flex gap-1">
                        {rated_string} <TimeRow time_info=time_info.into() />
                        <Show when=move || {
                            game().tournament.is_some()
                        }>
                            {t!(i18n, game.played_in)}
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
                            <StatusIndicator username=game().white_player.username />
                            <ProfileLink
                                patreon=game().white_player.patreon
                                username=game().white_player.username
                            />
                        </div>
                        <br />
                        <Show when=is_finished fallback=move || { game().white_rating() }>
                            <RatingAndChange ratings side=Color::White />
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
                        <br />
                        <Show when=is_finished fallback=move || { game().black_rating() }>
                            <RatingAndChange ratings side=Color::Black />
                        </Show>
                    </div>
                </div>
                <div class="flex gap-1 justify-center items-center w-full">
                    {result_string} <Show when=is_finished>
                        <div>{conclusion}</div>
                    </Show>
                </div>
                <div class="flex gap-1 justify-between items-center w-full">
                    {history_string} <DownloadPgn game=game />
                </div>
            </div>
            <a
                class="absolute top-0 left-0 z-10 w-full h-full"
                href=format!("/game/{}", game().game_id)
            ></a>
        </article>
    }
}
