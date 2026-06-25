use crate::{
    common::with_class,
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext},
};
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_use::{
    use_interval_fn_with_options,
    use_timeout_fn,
    UseIntervalFnOptions,
    UseTimeoutFnReturn,
};
use shared_types::{GameId, ReadyUser};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[component]
pub fn TournamentReadyPopup(
    ready_signal: RwSignal<HashMap<GameId, Vec<ReadyUser>>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();
    let params = use_params_map();
    let countdown = RwSignal::new(30);
    let closed_popups = RwSignal::new(HashSet::<(GameId, Uuid)>::new());

    let current_popup_candidate = Memo::new(move |_| {
        let current_user_id = auth_context.user.with(|user| user.as_ref().map(|u| u.id))?;

        ready_signal.with(|ready_map| {
            if ready_map.is_empty() {
                return None;
            }

            closed_popups.with(|closed_set| {
                ready_map
                    .iter()
                    .flat_map(|(game_id, users)| {
                        users.iter().map(move |ready_user| (game_id, ready_user))
                    })
                    .find_map(|(game_id, ready_user)| {
                        (ready_user.proposer_id != current_user_id
                            && current_user_id == ready_user.opponent_id
                            && !closed_set.contains(&(game_id.clone(), ready_user.proposer_id)))
                        .then(|| {
                            (
                                game_id.clone(),
                                ready_user.proposer_id,
                                ready_user.proposer_username.clone(),
                            )
                        })
                    })
            })
        })
    });

    Effect::new(move |_| {
        ready_signal.with(|ready_map| {
            closed_popups.update(|closed_set| {
                closed_set.retain(|(game_id, user_id)| {
                    ready_map
                        .get(game_id)
                        .map(|users| {
                            users
                                .iter()
                                .any(|ready_user| ready_user.proposer_id == *user_id)
                        })
                        .unwrap_or(false)
                });
            });
        });
    });

    let is_visible = Signal::derive(move || current_popup_candidate.with(|opt| opt.is_some()));

    let opponent_name = Signal::derive(move || {
        current_popup_candidate.with(|opt| {
            opt.as_ref()
                .map(|(_, _, username)| username.clone())
                .unwrap_or_else(|| "Unknown Player".to_string())
        })
    });

    let current_game_id = Signal::derive(move || params.get().get("nanoid").unwrap_or_default());

    let is_on_game_page = Signal::derive(move || {
        current_popup_candidate.with(|opt| {
            opt.as_ref()
                .map(|(game_id, _, _)| current_game_id.get() == game_id.0)
                .unwrap_or(false)
        })
    });

    let accept_game = move |_| {
        if let Some(game_id) = current_popup_candidate
            .with(|opt| {
                opt.as_ref().map(|(game_id, opponent_id, _)| {
                    api.get().tournament_game_start(game_id.clone());
                    closed_popups.update(|set| {
                        set.insert((game_id.clone(), *opponent_id));
                    });
                    game_id.clone()
                })
            })
            .filter(|_| !is_on_game_page.get_untracked())
        {
            let navigate = use_navigate();
            navigate(&format!("/game/{}", game_id.0), Default::default());
        }
    };

    let close_popup = move |_| {
        current_popup_candidate.with(|opt| {
            if let Some((game_id, opponent_id, _)) = opt.as_ref() {
                closed_popups.update(|set| {
                    set.insert((game_id.clone(), *opponent_id));
                });
            }
        });
    };

    let view_game = move |_| {
        if let Some(game_id) = current_popup_candidate.with(|opt| {
            opt.as_ref().map(|(game_id, opponent_id, _)| {
                closed_popups.update(|set| {
                    set.insert((game_id.clone(), *opponent_id));
                });
                game_id.clone()
            })
        }) {
            let navigate = use_navigate();
            navigate(&format!("/game/{}", game_id.0), Default::default());
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
            current_popup_candidate.with(|opt| {
                if let Some((game_id, opponent_id, _)) = opt.as_ref() {
                    closed_popups.update(|set| {
                        set.insert((game_id.clone(), *opponent_id));
                    });
                }
            })
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
            with_class(
                "ui-modal-panel",
                format!(
                    "fixed left-1/2 top-1/2 z-50 w-full max-w-md -translate-x-1/2 -translate-y-1/2 p-6 mx-4 {}",
                    if is_visible.get() { "block" } else { "hidden" },
                ),
            )
        }>
            <div class="text-center">
                <div class="mb-4">
                    <h2 class="mb-2 text-xl font-bold text-gray-900 dark:text-white">
                        {t!(i18n, game.tournament_ready_title)}
                    </h2>
                    <p class="text-gray-700 dark:text-gray-300">
                        {t!(i18n, game.tournament_ready_opponent)} " " {opponent_name} " "
                        {t!(i18n, game.tournament_ready_message)}
                    </p>
                    <div class="mt-2 text-lg font-bold text-red-600 dark:text-red-400">
                        {move || format!("{}s", countdown.get())}
                    </div>
                </div>

                <div class="flex gap-4 justify-center">
                    <button on:click=accept_game class="ui-button ui-button-success ui-button-md">
                        {t!(i18n, game.tournament_ready_accept)}
                    </button>

                    <Show when=move || !is_on_game_page.get()>
                        <button on:click=view_game class="ui-button ui-button-primary ui-button-md">
                            {t!(i18n, game.tournament_ready_view_game)}
                        </button>
                    </Show>

                    <button on:click=close_popup class="ui-button ui-button-secondary ui-button-md">
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
                "fixed inset-0 z-40 bg-black/50 backdrop-blur-sm {}",
                if is_visible.get() { "block" } else { "hidden" },
            )
        }></div>
    }
}
