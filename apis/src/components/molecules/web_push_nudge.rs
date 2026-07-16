use crate::{
    i18n::*,
    providers::{AuthContext, AuthIdentity},
    pwa,
};
use leptos::{prelude::*, task::spawn_local};

#[component]
pub fn WebPushNudge(install_nudge_active: RwSignal<bool>) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let i18n = use_i18n();
    let dismissed = RwSignal::new(false);
    let busy = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let should_show = LocalResource::new(move || {
        let logged_in = matches!(auth.identity.get(), Some(AuthIdentity::User(_)));
        async move {
            if !logged_in
                || !pwa::supported()
                || pwa::permission_blocked()
                || pwa::nudge_dismissed()
            {
                return false;
            }
            if !pwa::push_available().await {
                return false;
            }
            !pwa::is_subscribed().await
        }
    });
    let show = Signal::derive(move || {
        !dismissed.get() && !install_nudge_active.get() && should_show.get().unwrap_or(false)
    });

    let enable = move |_| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match pwa::subscribe().await {
                Ok(()) => dismissed.set(true),
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    let dismiss = move |_| {
        pwa::dismiss_nudge();
        dismissed.set(true);
    };

    view! {
        <Show when=show>
            <div class="flex fixed inset-x-2 bottom-2 z-50 gap-3 items-center p-3 mx-auto w-auto max-w-md text-sm sm:right-4 sm:inset-x-auto sm:left-auto ui-panel">
                <div class="flex-1 min-w-0">
                    <p class="font-semibold text-gray-900 dark:text-gray-100">
                        {t!(i18n, notifications.nudge.title)}
                    </p>
                    <p class="text-gray-700 dark:text-gray-300">
                        {t!(i18n, notifications.nudge.body_before_link)}
                        <a href="/notifications" class="ui-text-link">
                            {t!(i18n, notifications.nudge.link)}
                        </a> {t!(i18n, notifications.nudge.body_after_link)}
                    </p>
                    <Show when=move || error.with(Option::is_some)>
                        <p class="mt-1 ui-field-error">{move || error.get().unwrap_or_default()}</p>
                    </Show>
                </div>
                <button
                    class="ui-button ui-button-primary ui-button-sm"
                    disabled=busy
                    on:click=enable
                >
                    {move || {
                        if busy.get() {
                            t_string!(i18n, notifications.nudge.enabling)
                        } else {
                            t_string!(i18n, notifications.nudge.enable)
                        }
                    }}
                </button>
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
