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
use shared_types::{GameStart, PrettyString, TimeInfo, TournamentId};

#[component]
pub fn GameRow(game: GameResponse) -> impl IntoView {
    let i18n = use_i18n();
    let white_rating = game.white_rating();
    let black_rating = game.black_rating();
    let ratings = StoredValue::new(RatingChangeInfo::from_game_response(&game));
    let board = game.create_state().board;
    let game_stored = StoredValue::new(game.clone());
    let game_status = StoredValue::new(game.game_status);
    let turn = game.turn;
    let white_player = game.white_player;
    let black_player = game.black_player;
    let is_tournament = game.tournament.is_some();
    let (tournament_id, tournament_name) = match game.tournament {
        Some(t) => (t.tournament_id, t.name),
        None => (TournamentId::default(), String::new()),
    };
    let conclusion = game.conclusion;
    let finished = game.finished;
    let history = game.history;
    let time_info = TimeInfo {
        mode: game.time_mode,
        base: game.time_base,
        increment: game.time_increment,
    };
    let rated_string = move || match game.rated {
        true => t_string!(i18n, game.rated),
        false => t_string!(i18n, game.casual),
    };

    let ago = move || match finished {
        true => {
            let time = Utc::now().signed_duration_since(game.updated_at);
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
            let time = Utc::now().signed_duration_since(game.created_at);
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

    let status_string = move || {
        let status = match game_status.get_value() {
            GameStatus::NotStarted => {
                if game.game_start == GameStart::Ready {
                    t_string!(i18n, game.start_when.both_agree).to_string()
                } else {
                    "Not started".to_string()
                }
            }
            GameStatus::Adjudicated => game.tournament_game_result.to_string(),
            GameStatus::InProgress => "Playing now".to_string(),
            GameStatus::Finished(ref res) => match res {
                GameResult::Winner(c) => GameResult::Winner(*c).to_string(),
                GameResult::Draw => GameResult::Draw.to_string(),
                _ => conclusion.to_string(),
            },
        };
        if finished {
            format!("{status} {}", conclusion.pretty_string())
        } else {
            status
        }
    };

    let player_class = |color: Color| {
        let base_class = "flex items-center truncate p-1 overflow-hidden justify-center max-w-full";
        let is_active = matches!(
            game_status.get_value(),
            GameStatus::InProgress | GameStatus::NotStarted
        );
        let is_current_turn = match color {
            Color::White => turn % 2 == 0,
            Color::Black => turn % 2 == 1,
        };
        
        if is_active && is_current_turn {
            format!("{base_class} border-2 border-pillbug-teal/30 rounded-lg")
        } else {
            base_class.to_string()
        }
    };
    view! {
        <article class="relative flex flex-col w-full min-h-[10rem] sm:min-h-[14rem] duration-300 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light hover:bg-blue-light hover:dark:bg-teal-900">
            <div class="flex justify-between items-start m-2">
                <div class="flex flex-col gap-1">
                    <p class="flex gap-1 truncate">{rated_string} <TimeRow time_info /></p>
                    <p class="text-sm opacity-75">{ago}</p>
                </div>
                <div class="z-50">
                    <DownloadPgn game=game_stored />
                </div>
            </div>

            <div class="flex flex-1 gap-4">
                <div class="flex-shrink-0 w-1/3 sm:w-2/5 max-w-[150px]">
                    <ThumbnailPieces board=StoredValue::new(board) />
                </div>
                <div class="flex flex-col flex-1 min-w-0">
                    <Show when=move || { is_tournament }>
                        <div class="flex pb-1 mb-1 border-b border-gray-200 dark:border-gray-700 md:place-content-center">
                            <p class="flex gap-1 text-sm truncate">
                                {t!(i18n, game.played_in)}
                                <a
                                    class="relative z-50 text-blue-500 truncate hover:underline"
                                    href=format!("/tournament/{}", tournament_id)
                                >
                                    {tournament_name.clone()}
                                </a>
                            </p>
                        </div>
                    </Show>
                    <p class="mb-2 font-semibold text-center">{status_string}</p>

                    <div class="flex gap-1 justify-between items-center mb-2 overflow-hidden">
                        <div class="flex flex-col items-center min-w-0 w-[45%] overflow-hidden">
                            <div class=player_class(Color::White)>
                                <StatusIndicator username=white_player.username.clone() />
                                <ProfileLink
                                    patreon=white_player.patreon
                                    username=white_player.username
                                    bot=white_player.bot
                                    attr:class="truncate z-50 mr-1 max-w-full"
                                />
                            </div>
                            <Show when=move || finished fallback=move || white_rating>
                                <RatingAndChange ratings side=Color::White/>
                            </Show>
                        </div>

                        <Icon icon=icondata::RiSwordOthersLine attr:class="flex-shrink-0 mx-1 text-sm" />

                        <div class="flex flex-col items-center min-w-0 w-[45%] overflow-hidden">
                            <div class=player_class(Color::Black)>
                                <ProfileLink
                                    username=black_player.username.clone()
                                    patreon=black_player.patreon
                                    bot=black_player.bot
                                    attr:class="truncate z-50 ml-1 max-w-full"
                                />
                                <StatusIndicator username=black_player.username />
                            </div>
                            <Show when=move || finished fallback=move || black_rating>
                                <RatingAndChange ratings side=Color::Black />
                            </Show>
                        </div>
                    </div>

                    <p class="pr-1 text-xs text-center opacity-75 sm:text-sm line-clamp-2">
                        {history_string}
                    </p>
                </div>
            </div>
            <a
                class="absolute inset-0 z-20"
                href=format!("/game/{}", game.game_id)
                aria-label="View game details"
            ></a>
        </article>
    }
}
