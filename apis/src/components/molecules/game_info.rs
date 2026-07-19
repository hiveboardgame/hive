use crate::{
    common::{
        format_game_rating,
        format_game_result,
        game_time_info,
        game_tournament_link,
        untimed_time_info,
    },
    components::molecules::time_row::TimeRow,
    i18n::*,
    providers::game_state::{GameStateStore, GameStateStoreFields},
};
use leptos::prelude::*;

#[component]
pub fn GameInfo(
    #[prop(optional)] extend_tw_classes: &'static str,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let game_state = expect_context::<GameStateStore>();
    let game_response = game_state.game_response();
    let has_game_response =
        Memo::new(move |_| game_response.with(|game_response| game_response.is_some()));
    let time_info = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .map(game_time_info)
                .unwrap_or_else(untimed_time_info)
        })
    });
    let rated = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .map(|game| game.rated)
                .unwrap_or_default()
        })
    });
    let rated_text = move || format_game_rating(i18n, rated());
    let tournament_info = Memo::new(move |_| {
        game_response.with(|game_response| game_response.as_ref().and_then(game_tournament_link))
    });
    let is_tournament = Memo::new(move |_| tournament_info().is_some());
    let tournament_name =
        Memo::new(move |_| tournament_info().map(|link| link.name).unwrap_or_default());
    let tournament_href =
        Memo::new(move |_| tournament_info().map(|link| link.href).unwrap_or_default());
    let tournament_label = Memo::new(move |_| {
        tournament_info()
            .map(|link| {
                if compact {
                    format!("in {}", link.name)
                } else {
                    format!("played in {}", link.name)
                }
            })
            .unwrap_or_default()
    });
    let compact_tournament_label = Memo::new(move |_| {
        tournament_info()
            .map(|link| format!("in {}", link.name))
            .unwrap_or_default()
    });
    let result_text = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .and_then(|game| format_game_result(i18n, game))
        })
    });

    let container_class = if compact { "contents" } else { "min-w-0" };
    let outer_class = move || {
        if compact {
            format!("contents {extend_tw_classes}")
        } else {
            extend_tw_classes.to_string()
        }
    };
    let metadata_class = if compact {
        "col-start-2 row-start-1 ml-auto flex min-w-0 flex-wrap items-center justify-end gap-x-1 gap-y-0 text-right text-xs leading-tight"
    } else {
        "flex min-w-0 flex-wrap items-center gap-x-1 gap-y-0.5"
    };
    let rated_class = "shrink-0";
    let result_class = if compact {
        "col-span-2 min-w-0 text-left text-xs leading-tight"
    } else {
        "min-w-[12rem] flex-1 leading-tight"
    };
    let inline_result_class = if compact {
        "shrink-0 max-w-full truncate text-right"
    } else {
        result_class
    };
    let tournament_link_class = if compact {
        "hidden min-w-0 max-w-[45vw] truncate pointer-events-auto text-gray-600 no-link-style hover:underline xs:inline-block dark:text-gray-300"
    } else {
        "min-w-0 max-w-full truncate pointer-events-auto"
    };
    let tournament_wrap_link_class = "col-span-2 block min-w-0 truncate text-left text-xs leading-tight text-gray-600 no-link-style hover:underline xs:hidden dark:text-gray-300";

    view! {
        <Show when=has_game_response>
            <div class=outer_class>
                <div class=container_class>
                    <div class=metadata_class>
                        <div class="shrink-0">
                            <TimeRow time_info extend_tw_classes="whitespace-nowrap" />
                        </div>
                        <div class=rated_class>{rated_text}</div>
                        <Show when=is_tournament>
                            <a
                                href=tournament_href
                                title=tournament_name
                                class=tournament_link_class
                            >
                                {tournament_label}
                            </a>
                        </Show>
                        <Show when=move || {
                            (!compact || !is_tournament()) && result_text().is_some()
                        }>
                            <div class=inline_result_class>{result_text}</div>
                        </Show>
                    </div>
                    <Show when=move || compact && is_tournament()>
                        <a
                            href=tournament_href
                            title=tournament_name
                            class=tournament_wrap_link_class
                        >
                            {compact_tournament_label}
                        </a>
                    </Show>
                    <Show when=move || { compact && is_tournament() && result_text().is_some() }>
                        <div class=result_class>{result_text}</div>
                    </Show>
                </div>
            </div>
        </Show>
    }
}
