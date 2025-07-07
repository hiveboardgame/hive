use crate::i18n::*;
use crate::providers::{ApiRequestsProvider, AuthContext};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_use::{
    use_interval_fn_with_options, use_timeout_fn, UseIntervalFnOptions, UseTimeoutFnReturn,
};
use shared_types::GameId;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[component]
pub fn TournamentReadyPopup(
    ready_signal: RwSignal<HashMap<GameId, Vec<(Uuid, String)>>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();
    let params = use_params_map();
    let countdown = RwSignal::new(30);
    let closed_popups = RwSignal::new(HashSet::<(GameId, Uuid)>::new());

    let current_popup_candidate = Memo::new(move |_| {
        let ready_map = ready_signal.get();
        let current_user_id = auth_context.user.get().map(|u| u.id);
        let closed_set = closed_popups.get();

        if let Some(user_id) = current_user_id {
            for (ready_game_id, ready_users) in ready_map.iter() {
                for (ready_user_id, ready_username) in ready_users {
                    if *ready_user_id != user_id {
                        let candidate_key = (ready_game_id.clone(), *ready_user_id);
                        if !closed_set.contains(&candidate_key) {
                            return Some((
                                ready_game_id.clone(),
                                *ready_user_id,
                                ready_username.clone(),
                            ));
                        }
                    }
                }
            }
        }
        None
    });

    Effect::new(move |_| {
        let ready_map = ready_signal.get();
        closed_popups.update(|closed_set| {
            closed_set.retain(|(game_id, user_id)| {
                ready_map
                    .get(game_id)
                    .map(|users| users.iter().any(|(id, _)| id == user_id))
                    .unwrap_or(false)
            });
        });
    });

    let is_visible = Signal::derive(move || current_popup_candidate.get().is_some());

    let opponent_name = Signal::derive(move || {
        current_popup_candidate
            .get()
            .map(|(_, _, username)| username)
            .unwrap_or_else(|| "Unknown Player".to_string())
    });

    let current_game_id = Signal::derive(move || params.get().get("nanoid").unwrap_or_default());

    let is_on_game_page = Signal::derive(move || {
        current_popup_candidate
            .get()
            .map(|(game_id, _, _)| current_game_id.get() == game_id.0)
            .unwrap_or(false)
    });

    let accept_game = move |_| {
        if let Some((game_id, opponent_id, _)) = current_popup_candidate.get() {
            let api = api.get();
            api.tournament_game_start(game_id.clone());

            closed_popups.update(|set| {
                set.insert((game_id.clone(), opponent_id));
            });

            if !is_on_game_page.get() {
                let navigate = use_navigate();
                navigate(&format!("/game/{}", game_id.0), Default::default());
            }
        }
    };

    let close_popup = move |_| {
        if let Some((game_id, opponent_id, _)) = current_popup_candidate.get() {
            closed_popups.update(|set| {
                set.insert((game_id, opponent_id));
            });
        }
    };

    let interval = use_interval_fn_with_options(
        move || {
            countdown.update(|c| {
                if *c > 0 {
                    *c -= 1;
                }
            });
        },
        1000,
        UseIntervalFnOptions::default().immediate(false),
    );

    let UseTimeoutFnReturn { start, stop, .. } = use_timeout_fn(
        move |_: ()| {
            if let Some((game_id, opponent_id, _)) = current_popup_candidate.get() {
                closed_popups.update(|set| {
                    set.insert((game_id, opponent_id));
                });
            }
        },
        30_000.0,
    );

    Effect::new(move |_| {
        if is_visible.get() {
            countdown.set(30);
            (interval.resume)();
            start(());
        } else {
            (interval.pause)();
            stop();
        }
    });

    view! {
        <div class=move || {
            format!(
                "fixed top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 z-50 bg-white dark:bg-gray-800 border-2 border-gray-300 dark:border-gray-600 rounded-lg shadow-2xl p-6 max-w-md w-full mx-4 {}",
                if is_visible.get() { "block" } else { "hidden" },
            )
        }>
            <div class="text-center">
                <div class="mb-4">
                    <h2 class="mb-2 text-xl font-bold text-gray-900 dark:text-white">
                        {t!(i18n, game.tournament_ready_title)}
                    </h2>
                    <p class="text-gray-700 dark:text-gray-300">
                        {t!(i18n, game.tournament_ready_opponent)} " " {move || opponent_name.get()}
                        " " {t!(i18n, game.tournament_ready_message)}
                    </p>
                    <div class="mt-2 text-lg font-bold text-red-600 dark:text-red-400">
                        {move || format!("{}s", countdown.get())}
                    </div>
                </div>

                <div class="flex gap-4 justify-center">
                    <button
                        on:click=accept_game
                        class="px-4 py-2 font-bold text-white bg-green-600 rounded transition-transform duration-300 transform hover:bg-green-700 active:scale-95"
                    >
                        {t!(i18n, game.tournament_ready_accept)}
                    </button>

                    <Show when=move || !is_on_game_page.get()>
                        <button
                            on:click=move |_| {
                                if let Some((game_id, opponent_id, _)) = current_popup_candidate.get() {
                                    closed_popups.update(|set| {
                                        set.insert((game_id.clone(), opponent_id));
                                    });
                                    let navigate = use_navigate();
                                    navigate(&format!("/game/{}", game_id.0), Default::default());
                                }
                            }
                            class="px-4 py-2 font-bold text-white bg-blue-600 rounded transition-transform duration-300 transform hover:bg-blue-700 active:scale-95"
                        >
                            {t!(i18n, game.tournament_ready_view_game)}
                        </button>
                    </Show>

                    <button
                        on:click=close_popup
                        class="px-4 py-2 font-bold text-white bg-gray-600 rounded transition-transform duration-300 transform hover:bg-gray-700 active:scale-95"
                    >
                        {t!(i18n, game.tournament_ready_close)}
                    </button>
                </div>

                <div class="mt-3 text-sm text-gray-500 dark:text-gray-400">
                    {t!(i18n, game.tournament_ready_timeout)}
                </div>
            </div>
        </div>

        <div class=move || {
            format!(
                "fixed inset-0 bg-black bg-opacity-50 z-40 {}",
                if is_visible.get() { "block" } else { "hidden" },
            )
        }></div>
    }
}
