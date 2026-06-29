use crate::{i18n::*, pwa};
use leptos::prelude::*;

#[component]
pub fn InstallNudge(active: RwSignal<bool>) -> impl IntoView {
    let i18n = use_i18n();

    let install = move |_| {
        pwa::prompt_install();
        active.set(false);
    };
    let dismiss = move |_| {
        pwa::dismiss_install_nudge();
        active.set(false);
    };
    let is_ios = move || pwa::ios_needs_install();

    view! {
        <Show when=active>
            <div class="flex fixed inset-x-2 bottom-2 z-50 gap-3 items-center py-3 px-4 mx-auto w-auto max-w-md text-sm rounded-lg border shadow bg-stone-300 border-stone-400 dark:bg-slate-800 dark:border-slate-600">
                <span class="text-xl">"📲"</span>
                <div class="flex-1 min-w-0">
                    <p class="font-semibold dark:text-white">
                        {t!(i18n, notifications.install.title)}
                    </p>
                    <Show when=is_ios>
                        <p class="text-gray-700 dark:text-gray-300">
                            {t!(i18n, notifications.install.ios_hint)}
                        </p>
                    </Show>
                </div>
                <Show when=move || !is_ios()>
                    <button
                        class="py-1.5 px-3 font-bold text-white whitespace-nowrap rounded shadow bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal"
                        on:click=install
                    >
                        {t!(i18n, notifications.install.button)}
                    </button>
                </Show>
                <button
                    class="px-2 text-xl leading-none text-gray-500 hover:text-gray-800 dark:hover:text-white"
                    aria-label=move || t_string!(i18n, notifications.nudge.dismiss).to_string()
                    on:click=dismiss
                >
                    "×"
                </button>
            </div>
        </Show>
    }
}
