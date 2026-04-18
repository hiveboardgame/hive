use crate::i18n::*;
use leptos::prelude::*;
use shared_types::GameThread;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameThreadToggleSize {
    Compact,
    Roomy,
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
        }
    }
}

#[component]
pub fn GameThreadToggle(
    selected: RwSignal<GameThread>,
    spectators_enabled: Signal<bool>,
    size: GameThreadToggleSize,
) -> impl IntoView {
    let i18n = use_i18n();

    view! {
        <div class=size.container_class()>
            <button
                type="button"
                class=move || {
                    format!(
                        "{} {}",
                        size.button_base_class(),
                        if selected.get() == GameThread::Players {
                            "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
                        } else {
                            "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
                        },
                    )
                }
                on:click=move |_| selected.set(GameThread::Players)
            >
                {t!(i18n, messages.chat.players)}
            </button>
            <button
                type="button"
                disabled=move || !spectators_enabled.get()
                class=move || {
                    format!(
                        "{} {}",
                        size.button_base_class(),
                        if selected.get() == GameThread::Spectators {
                            "bg-slate-400 dark:bg-button-twilight text-gray-900 dark:text-gray-100"
                        } else if spectators_enabled.get() {
                            "bg-transparent text-gray-700 dark:text-gray-300 hover:bg-black/5 dark:hover:bg-white/5"
                        } else {
                            "bg-transparent text-gray-500 dark:text-gray-500 cursor-not-allowed"
                        },
                    )
                }
                on:click=move |_| selected.set(GameThread::Spectators)
            >
                {t!(i18n, messages.chat.spectators)}
            </button>
        </div>
    }
}
