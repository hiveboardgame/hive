use crate::{
    common::{format_game_result, format_tournament_datetime, with_class, RatingChangeInfo},
    components::{
        atoms::color_hex::ColorHex,
        molecules::{
            rating_and_change::RatingAndChange,
            thumbnail_pieces::ThumbnailPieces,
            time_row::TimeRow,
        },
    },
    i18n::*,
    responses::{GameResponse, UserResponse},
};
use hive_lib::Color;
use leptos::prelude::*;

#[component]
pub fn GamePreview(
    game: GameResponse,
    #[prop(optional)] drawer: bool,
    #[prop(optional)] show_time: bool,
    #[prop(optional)] show_tournament_date: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let board = game.create_state().board;
    let time_control = game.time_control();
    let speed = game.speed;
    let finished = game.finished;
    let rated = game.rated;
    let game_id = game.game_id.clone();
    let tournament_date = StoredValue::new(show_tournament_date.then(|| {
        format_tournament_datetime(
            game.finished_at
                .or(game.last_interaction)
                .unwrap_or(game.created_at),
        )
    }));
    let ratings = StoredValue::new(RatingChangeInfo::from_game_response(&game));
    let result_game = StoredValue::new(game.clone());
    let white_player = StoredValue::new(game.white_player);
    let black_player = StoredValue::new(game.black_player);
    let username = move |player: StoredValue<UserResponse>| {
        player.with_value(|player| {
            if player.deleted {
                t_string!(i18n, profile.deleted_user).to_string()
            } else {
                player.username.clone()
            }
        })
    };
    let rating = move |player: StoredValue<UserResponse>| {
        player.with_value(|player| player.ratings.get(&speed).expect("Has a rating").rating)
    };

    view! {
        <article class=with_class(
            "ui-card-row",
            if drawer {
                "relative flex w-full min-w-0 flex-col items-center overflow-hidden"
            } else {
                "relative m-2 flex w-60 max-w-full shrink-0 flex-col items-center overflow-hidden align-top lg:mt-0 lg:mr-4 lg:mb-4 lg:ml-0 lg:inline-flex 2xl:m-2"
            },
        )>
            {if drawer {
                view! {
                    <div class="grid gap-1 py-1.5 px-2 w-full text-sm">
                        {[Color::White, Color::Black]
                            .into_iter()
                            .map(|side| {
                                let player = if side == Color::White {
                                    white_player
                                } else {
                                    black_player
                                };
                                view! {
                                    <div class="grid gap-1.5 items-center h-5 grid-cols-[1rem_minmax(0,1fr)_auto]">
                                        <ColorHex color=Signal::derive(move || side) />
                                        <span
                                            class="font-medium truncate"
                                            title=move || username(player)
                                        >
                                            {move || username(player)}
                                        </span>
                                        <span class="flex gap-1 items-center text-xs shrink-0">
                                            {if finished {
                                                view! { <RatingAndChange ratings side /> }.into_any()
                                            } else {
                                                view! {
                                                    <span class="italic text-gray-500">
                                                        {move || rating(player)}
                                                    </span>
                                                }
                                                    .into_any()
                                            }}
                                        </span>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                }
                    .into_any()
            } else {
                view! {
                    <div class="grid gap-1 items-center py-1.5 px-2 w-full text-sm grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]">
                        <div class="flex gap-1 justify-end items-center min-w-0">
                            <span class="font-medium truncate">
                                {move || username(white_player)}
                            </span>
                            <Show
                                when=move || finished
                                fallback=move || {
                                    view! {
                                        <span class="text-xs italic text-gray-500 dark:text-gray-400 shrink-0">
                                            {move || rating(white_player)}
                                        </span>
                                    }
                                }
                            >
                                <span class="flex gap-1 items-center text-xs shrink-0">
                                    <RatingAndChange ratings side=Color::White />
                                </span>
                            </Show>
                        </div>
                        <span class="text-xs text-gray-500 dark:text-gray-400">
                            {t!(i18n, tournaments.view.common.versus)}
                        </span>
                        <div class="flex gap-1 items-center min-w-0">
                            <span class="font-medium truncate">
                                {move || username(black_player)}
                            </span>
                            <Show
                                when=move || finished
                                fallback=move || {
                                    view! {
                                        <span class="text-xs italic text-gray-500 dark:text-gray-400 shrink-0">
                                            {move || rating(black_player)}
                                        </span>
                                    }
                                }
                            >
                                <span class="flex gap-1 items-center text-xs shrink-0">
                                    <RatingAndChange ratings side=Color::Black />
                                </span>
                            </Show>
                        </div>
                    </div>
                }
                    .into_any()
            }} <Show when=move || finished || drawer>
                <p
                    class=if drawer {
                        "px-2 mb-1 h-10 text-sm leading-5 text-center line-clamp-2"
                    } else {
                        "px-2 pb-1 text-sm text-center"
                    }
                    title=move || {
                        result_game
                            .with_value(|game| format_game_result(i18n, game))
                            .unwrap_or_default()
                    }
                >
                    {move || {
                        result_game
                            .with_value(|game| format_game_result(i18n, game))
                            .unwrap_or_default()
                    }}
                </p>
            </Show> <div class="overflow-hidden w-full aspect-square shrink-0">
                <ThumbnailPieces board=StoredValue::new(board) />
            </div> <Show when=move || show_time || tournament_date.with_value(Option::is_some)>
                <div class="flex flex-wrap gap-x-2 justify-center items-center py-1.5 px-2 w-full text-xs text-center">
                    <Show when=move || show_time>
                        <span class="uppercase">
                            {move || {
                                if rated {
                                    t_string!(i18n, game.rated).to_string()
                                } else {
                                    t_string!(i18n, game.casual).to_string()
                                }
                            }}
                        </span>
                        <TimeRow time_control extend_tw_classes="text-xs" />
                    </Show>
                    {move || {
                        tournament_date
                            .with_value(Clone::clone)
                            .map(|date| {
                                view! {
                                    <time class="text-gray-500 dark:text-gray-400">{date}</time>
                                }
                            })
                    }}
                </div>
            </Show>
            <a
                class="absolute inset-0 z-10"
                href=format!("/game/{}", game_id)
                title=move || {
                    result_game
                        .with_value(|game| format_game_result(i18n, game))
                        .unwrap_or_default()
                }
                aria-label=move || {
                    format!(
                        "{} {} {}",
                        username(white_player),
                        t_string!(i18n, tournaments.view.common.versus),
                        username(black_player),
                    )
                }
            ></a>
        </article>
    }
}

#[component]
pub fn GamePreviews(
    #[prop(into)] games: Signal<Vec<GameResponse>>,
    #[prop(optional)] show_time: bool,
    #[prop(optional)] show_tournament_date: bool,
    #[prop(optional)] nowrap: bool,
) -> impl IntoView {
    let class = if nowrap {
        "flex flex-row flex-nowrap min-w-max"
    } else {
        "flex flex-row flex-wrap justify-center w-full min-w-0 max-w-full lg:block 2xl:flex"
    };

    view! {
        <div class=class>
            <For each=games key=|game| { (game.game_id.clone(), game.updated_at) } let:game>
                <GamePreview game show_time show_tournament_date />
            </For>
        </div>
    }
}
