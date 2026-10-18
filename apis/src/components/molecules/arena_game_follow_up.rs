use crate::{
    functions::tournaments::{get_arena_pairing_status, PauseArena},
    hooks::{arena_clock::use_ticking_now, websocket_refresh::use_websocket_refresh},
    i18n::*,
    providers::{games::GamesSignal, AuthContext, AuthIdentity},
};
use chrono::{Duration, Utc};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use shared_types::{tournament::arena::PairingIntent, TournamentId};

#[component]
pub fn ArenaGameFollowUp(tournament_id: TournamentId) -> impl IntoView {
    let now = use_ticking_now();
    let i18n = use_i18n();
    let pause = ServerAction::<PauseArena>::new();
    let pause_id = StoredValue::new(tournament_id);
    let navigate = use_navigate();
    let arena_path = format!("/tournament/{}", pause_id.get_value());
    let identity = expect_context::<AuthContext>().identity;
    let games = expect_context::<GamesSignal>();
    let status = Resource::new(
        move || (identity.get(), pause.version().get()),
        move |(identity, _)| {
            let tournament_id = pause_id.get_value();
            async move {
                let Some(user_id) = identity.and_then(AuthIdentity::user_id) else {
                    return Ok(None);
                };
                get_arena_pairing_status(tournament_id)
                    .await
                    .map(|status| status.map(|status| (user_id, status)))
            }
        },
    );
    use_websocket_refresh(
        move || {
            pause.value().set(None);
            status.refetch();
        },
        false,
    );
    view! {
        <div class="flex flex-wrap gap-y-1 gap-x-2 justify-center items-center py-1 px-1 w-full">
            <button
                type="button"
                class="ui-button ui-button-primary ui-button-sm"
                on:click=move |_| navigate(&arena_path, Default::default())
            >
                {t!(i18n, game.arena.back_to_arena)}
            </button>
            <Transition>
                {move || {
                    let current_status = move || {
                        status
                            .get()
                            .and_then(Result::ok)
                            .flatten()
                            .filter(|(user_id, _)| {
                                identity.get().and_then(AuthIdentity::user_id) == Some(*user_id)
                            })
                    };
                    let pairing_deadline = Memo::new(move |_| {
                        current_status()
                            .map(|(_, status)| {
                                Utc::now() + Duration::seconds(status.pairing_seconds_left)
                            })
                    });
                    let pairing_open = move || {
                        pairing_deadline.get().is_some_and(|closes_at| now.get() < closes_at)
                    };
                    let paused = move || {
                        current_status().map(|(_, status)| status.intent == PairingIntent::Paused)
                    };
                    let active_game = Memo::new(move |_| {
                        let user_id = identity.get().and_then(AuthIdentity::user_id)?;
                        games
                            .own
                            .with(|games| {
                                games
                                    .realtime
                                    .values()
                                    .find(|game| {
                                        !game.finished
                                            && (game.white_player.uid == user_id
                                                || game.black_player.uid == user_id)
                                            && game
                                                .tournament
                                                .as_ref()
                                                .is_some_and(|arena| {
                                                    arena.tournament_id == pause_id.get_value()
                                                })
                                    })
                                    .map(|game| game.game_id.clone())
                            })
                            .or_else(|| current_status().and_then(|(_, status)| status.active_game))
                    });
                    view! {
                        {move || {
                            active_game
                                .get()
                                .map(|game_id| {
                                    view! {
                                        <a
                                            class="ui-button ui-button-primary ui-button-sm"
                                            href=format!("/game/{game_id}")
                                        >
                                            {t!(i18n, tournaments.view.arena.current_game)}
                                        </a>
                                    }
                                })
                        }}
                        <Show when=move || pairing_open() && paused() == Some(false)>
                            <button
                                type="button"
                                class="ui-button ui-button-secondary ui-button-sm"
                                disabled=move || pause.pending().get()
                                on:click=move |_| {
                                    pause
                                        .dispatch(PauseArena {
                                            tournament_id: pause_id.get_value(),
                                        });
                                }
                            >
                                {t!(i18n, tournaments.arena.pause)}
                            </button>
                        </Show>
                        <Show when=move || {
                            pairing_open() && paused().is_some() && active_game.get().is_none()
                        }>
                            <p
                                class="w-full text-xs text-center text-gray-600 dark:text-gray-300"
                                aria-live="polite"
                            >
                                {move || {
                                    if paused() == Some(true) {
                                        t_string!(i18n, game.arena.pairing_paused)
                                    } else {
                                        t_string!(i18n, game.arena.return_for_pairing)
                                    }
                                }}
                            </p>
                        </Show>
                        {move || {
                            let failed = pause.value().get().is_some_and(|result| result.is_err())
                                || status.get().is_some_and(|result| result.is_err());
                            failed
                                .then(|| {
                                    view! {
                                        <p
                                            class="w-full text-xs text-center text-red-700 dark:text-red-300"
                                            role="alert"
                                        >
                                            {t!(i18n, messages.errors.request_unavailable)}
                                        </p>
                                    }
                                })
                        }}
                    }
                }}
            </Transition>
        </div>
    }
}
