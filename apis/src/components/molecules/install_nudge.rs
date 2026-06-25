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
            <div class="flex fixed inset-x-2 bottom-2 z-50 gap-3 items-center p-3 mx-auto w-auto max-w-md text-sm ui-panel">
                <div class="flex-1 min-w-0">
                    <p class="font-semibold text-gray-900 dark:text-gray-100">
                        {t!(i18n, notifications.install.title)}
                    </p>
                    <Show when=is_ios>
                        <p class="ui-field-helper">{t!(i18n, notifications.install.ios_hint)}</p>
                    </Show>
                </div>
                <Show when=move || !is_ios()>
                    <button class="ui-button ui-button-primary ui-button-sm" on:click=install>
                        {t!(i18n, notifications.install.button)}
                    </button>
                </Show>
                <button
                    class="ui-button ui-button-ghost ui-button-icon-sm"
                    aria-label=move || t_string!(i18n, notifications.nudge.dismiss).to_string()
                    on:click=dismiss
                >
                    "×"
                </button>
            </div>
        </Show>
    }
}
