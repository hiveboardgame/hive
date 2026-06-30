use crate::{
    i18n::*,
    providers::{game_state::GameStateSignal, AuthContext},
};
use leptos::prelude::*;
use shared_types::{GameChatCapabilities, GameThread};

//TODO: Not sure I like this solution
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
                "flex shrink-0 border-b border-black/10 bg-odd-light/80 dark:border-white/10 dark:bg-surface-muted"
            }
            Self::Roomy => {
                "ui-setting-group grid grid-cols-2 gap-1 shrink-0"
            }
            Self::Route => {
                "inline-flex min-w-0 flex-wrap items-center gap-2"
            }
        }
    }

    const fn button_base_class(self) -> &'static str {
        match self {
            Self::Compact => "ui-board-tab-trigger cursor-pointer",
            Self::Roomy => "ui-choice ui-choice-sm",
            Self::Route => "ui-button ui-button-sm",
        }
    }
}

fn segment_class(
    size: GameThreadToggleSize,
    selected: GameThread,
    thread: GameThread,
    enabled: bool,
) -> String {
    let state_class = if size == GameThreadToggleSize::Compact {
        if selected == thread {
            "ui-segmented-active hover:bg-button-dawn dark:hover:bg-button-twilight"
        } else if enabled {
            "hover:bg-blue-light/70 dark:hover:bg-pillbug-teal/15"
        } else {
            "cursor-not-allowed opacity-50"
        }
    } else if size == GameThreadToggleSize::Route {
        if selected == thread {
            "ui-button-primary"
        } else if enabled {
            "ui-button-secondary"
        } else {
            "ui-button-secondary cursor-not-allowed opacity-50"
        }
    } else if selected == thread {
        "ui-choice-active"
    } else if enabled {
        "ui-choice-inactive"
    } else {
        "ui-choice-inactive cursor-not-allowed opacity-50"
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
        <div class=size.container_class() role="group">
            <button
                type="button"
                disabled=move || !players_enabled.get()
                aria-pressed=move || (selected.get() == GameThread::Players).to_string()
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
                aria-pressed=move || (selected.get() == GameThread::Spectators).to_string()
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
