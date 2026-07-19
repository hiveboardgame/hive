use crate::{
    common::{
        format_game_rating,
        format_game_result,
        game_time_info,
        game_tournament_link,
        untimed_time_info,
    },
    components::molecules::{time_row::TimeRow, user_with_rating::UserWithRating},
    i18n::*,
    providers::game_state::{GameStateStore, GameStateStoreFields},
};
use hive_lib::Color;
use leptos::prelude::*;

#[component]
pub fn GameDetailsPanel() -> impl IntoView {
    let i18n = use_i18n();
    let game_state = expect_context::<GameStateStore>();
    let game_response = game_state.game_response();
    let has_game_response =
        Memo::new(move |_| game_response.with(|game_response| game_response.is_some()));
    let details_class = move || {
        format!(
            "group select-none shrink-0 ui-board-side-panel {}",
            if has_game_response() { "" } else { "hidden" },
        )
    };
    let time_info = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .map(|game| (game_time_info(game), game.rated))
        })
    });
    let time_row_info = Memo::new(move |_| {
        time_info()
            .map(|(time, _)| time)
            .unwrap_or_else(untimed_time_info)
    });
    let rated_text = move || {
        time_info()
            .map(|(_, rated)| format_game_rating(i18n, rated))
            .unwrap_or_default()
    };
    let result_text = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .and_then(|game| format_game_result(i18n, game))
        })
    });
    let date_text = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .map(|game| game.created_at.format("%Y-%m-%d").to_string())
                .unwrap_or_default()
        })
    });
    let tournament_info = Memo::new(move |_| {
        game_response.with(|game_response| game_response.as_ref().and_then(game_tournament_link))
    });
    let tournament_name =
        Memo::new(move |_| tournament_info().map(|link| link.name).unwrap_or_default());
    let tournament_href =
        Memo::new(move |_| tournament_info().map(|link| link.href).unwrap_or_default());
    let summary_title = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .map(|game| {
                    format!(
                        "{} vs {}",
                        game.white_player.username, game.black_player.username
                    )
                })
                .unwrap_or_else(|| "Game details".to_string())
        })
    });
    let game_href = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .map(|game| format!("/game/{}", game.game_id))
                .unwrap_or_default()
        })
    });

    view! {
        <details class=details_class>
            <summary class="grid relative gap-3 items-center px-3 text-sm font-bold transition-colors duration-200 cursor-pointer grid-cols-[auto_minmax(0,1fr)] min-h-10 border-black/10 group-open:border-b [&::-webkit-details-marker]:hidden dark:border-white/10 dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70">
                <span class="inline-flex justify-center items-center text-xs rounded border transition-colors size-5 border-black/10 bg-odd-light dark:border-white/10 dark:bg-surface-muted">
                    <span class="group-open:hidden">"+"</span>
                    <span class="hidden group-open:inline">"-"</span>
                </span>
                <a
                    href=game_href
                    title=summary_title
                    class="relative z-20 justify-self-end min-w-0 max-w-full text-right text-gray-900 dark:text-gray-100 hover:underline truncate no-link-style"
                >
                    {summary_title}
                </a>
            </summary>
            <div class="flex flex-col gap-2 p-3 text-sm">
                <div class=move || {
                    format!(
                        "flex flex-wrap gap-x-3 gap-y-1 items-center min-w-0 text-xs leading-tight {}",
                        if time_info().is_none() { "hidden" } else { "" },
                    )
                }>
                    <TimeRow time_info=time_row_info extend_tw_classes="whitespace-nowrap" />
                    <span class="text-gray-700 dark:text-gray-200">{rated_text}</span>
                    <span class="text-gray-700 whitespace-nowrap dark:text-gray-200">
                        {date_text}
                    </span>
                    <span class=move || {
                        format!(
                            "min-w-0 font-semibold text-gray-900 dark:text-gray-100 {}",
                            if result_text().is_some() { "" } else { "hidden" },
                        )
                    }>{result_text}</span>
                </div>
                <div class="grid gap-2">
                    <div class="flex items-center min-w-0 leading-tight">
                        <UserWithRating side=Color::White vertical=true />
                    </div>
                    <div class="flex items-center min-w-0 leading-tight">
                        <UserWithRating side=Color::Black vertical=true />
                    </div>
                </div>
                <div class=move || {
                    format!(
                        "flex gap-2 items-center min-w-0 text-xs leading-tight {}",
                        if tournament_info().is_none() { "hidden" } else { "" },
                    )
                }>
                    <span class="text-gray-700 whitespace-nowrap dark:text-gray-200 shrink-0">
                        "Played in"
                    </span>
                    <a
                        href=tournament_href
                        title=tournament_name
                        class="flex-1 min-w-0 font-semibold hover:underline truncate text-pillbug-teal no-link-style"
                    >
                        {tournament_name}
                    </a>
                </div>
            </div>
        </details>
    }
}
