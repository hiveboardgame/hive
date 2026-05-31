use crate::{
    i18n::*,
    providers::{game_state::GameStateSignal, AuthContext},
};
use leptos::prelude::*;
use shared_types::{GameChatCapabilities, GameThread};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameThreadToggleSize {
    Compact,
    Roomy,
    Route,
}

#[derive(Copy, Clone)]
pub struct EmbeddedGameChatState {
    pub access: Signal<GameChatCapabilities>,
    pub selected_thread: RwSignal<GameThread>,
    pub explicit_thread: Signal<Option<GameThread>>,
}

pub fn use_embedded_game_chat_state() -> EmbeddedGameChatState {
    let game_state = expect_context::<GameStateSignal>();
    let auth_context = expect_context::<AuthContext>();
    let access = Signal::derive(move || {
        let is_player = auth_context.user.with(|user| {
            user.as_ref().is_some_and(|user| {
                game_state.signal.with(|game| {
                    game.white_id == Some(user.user.uid) || game.black_id == Some(user.user.uid)
                })
            })
        });
        let finished = game_state.signal.with(|game| {
            game.game_response
                .as_ref()
                .is_some_and(|game| game.finished)
        });
        GameChatCapabilities::new(is_player, finished)
    });
    let selected_thread = RwSignal::new(GameThread::Players);
    let explicit_thread = Signal::derive(move || {
        access
            .get()
            .can_toggle_embedded_threads()
            .then_some(selected_thread.get())
    });

    EmbeddedGameChatState {
        access,
        selected_thread,
        explicit_thread,
    }
}

impl GameThreadToggleSize {
    const fn container_class(self) -> &'static str {
        match self {
            Self::Compact => {
                "flex gap-0.5 p-1 border-b shrink-0 border-black/30 bg-inherit dark:border-white/30"
            }
            Self::Roomy => {
                "flex gap-0.5 p-1 mb-1 bg-gray-100 rounded-t border-b border-gray-300 dark:border-gray-600 shrink-0 dark:bg-gray-800/50"
            }
            Self::Route => {
                "flex p-0.5 bg-gray-100 rounded-lg border border-gray-300 dark:bg-gray-800 dark:border-gray-600"
            }
        }
    }

    const fn button_base_class(self) -> &'static str {
        match self {
            Self::Compact => {
                "flex-1 px-2 py-1 text-xs font-medium rounded border border-transparent transition-colors"
            }
            Self::Roomy => {
                "flex-1 px-3 py-2 text-sm font-medium rounded border border-transparent transition-colors"
            }
            Self::Route => {
                "no-link-style flex-1 px-3 py-1.5 text-sm font-medium rounded-md transition-colors text-center"
            }
        }
    }
}

fn segment_class(
    size: GameThreadToggleSize,
    selected: GameThread,
    thread: GameThread,
    enabled: bool,
) -> String {
    let state_class = if selected == thread {
        "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
    } else if enabled {
        "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
    } else {
        "bg-transparent text-gray-500 dark:text-gray-500 cursor-not-allowed"
    };
    format!("{} {}", size.button_base_class(), state_class)
}

#[component]
pub fn GameThreadToggle(
    selected: RwSignal<GameThread>,
    #[prop(optional, into)] players_enabled: Option<Signal<bool>>,
    spectators_enabled: Signal<bool>,
    size: GameThreadToggleSize,
    #[prop(optional)] on_select: Option<Callback<GameThread>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let players_enabled = players_enabled.unwrap_or_else(|| Signal::derive(|| true));
    let on_select = StoredValue::new(on_select);
    let select_thread = move |thread| {
        selected.set(thread);
        if let Some(on_select) = on_select.get_value() {
            on_select.run(thread);
        }
    };

    view! {
        <div class=size.container_class()>
            <button
                type="button"
                disabled=move || !players_enabled.get()
                class=move || {
                    segment_class(size, selected.get(), GameThread::Players, players_enabled.get())
                }
                on:click=move |_| select_thread(GameThread::Players)
            >
                {t!(i18n, messages.chat.players)}
            </button>
            <button
                type="button"
                disabled=move || !spectators_enabled.get()
                class=move || {
                    segment_class(
                        size,
                        selected.get(),
                        GameThread::Spectators,
                        spectators_enabled.get(),
                    )
                }
                on:click=move |_| select_thread(GameThread::Spectators)
            >
                {t!(i18n, messages.chat.spectators)}
            </button>
        </div>
    }
}
