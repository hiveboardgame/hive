use leptos::prelude::*;
use leptos_use::{use_interval_fn_with_options, utils::Pausable, UseIntervalFnOptions};

use crate::{
    i18n::*,
    providers::{AlertType, AlertsContext},
};

#[component]
pub fn Alert() -> impl IntoView {
    let i18n = use_i18n();
    let alerts = expect_context::<AlertsContext>();
    let visible = move || alerts.last_alert.with(Option::is_some);
    let Pausable { pause, resume, .. } = use_interval_fn_with_options(
        move || {
            alerts.last_alert.update(|v| *v = None);
        },
        4000,
        UseIntervalFnOptions::default().immediate(false),
    );
    let close = move |_| {
        alerts.last_alert.update(|v| *v = None);
    };
    let color = move || match (alerts.last_alert)() {
        Some(AlertType::Error(_)) => "bg-red-100 border-ladybug-red",
        Some(AlertType::Warn(_)) => "bg-yellow-100 border-orange-twilight",
        Some(AlertType::Notification(_)) => "bg-blue-100 border-pillbug-teal",
        None => "",
    };
    let label = move || match (alerts.last_alert)() {
        Some(AlertType::Error(_)) => t_string!(i18n, messages.chat.alert_error).to_string(),
        Some(AlertType::Warn(_)) => t_string!(i18n, messages.chat.alert_warning).to_string(),
        Some(AlertType::Notification(_)) => {
            t_string!(i18n, messages.chat.alert_notification).to_string()
        }
        None => String::new(),
    };

    Effect::new(move |_| {
        if (alerts.last_alert)().is_some() {
            resume();
        } else {
            pause()
        }
    });
    view! {
        <div
            class=move || {
                format!(
                    "top-4 w-3/5 lg:w-4/5 border {} text-black px-4 py-3 rounded fixed left-1/2 -translate-x-1/2 z-50 {}",
                    color(),
                    if visible() { "block" } else { "hidden" },
                )
            }

            role="alert"
        >
            <strong class="font-bold">{label}</strong>
            " "
            <span class="block">
                {move || {
                    if visible() {
                        let Some(alert) = (alerts.last_alert)() else {
                            return String::new();
                        };
                        alert.to_string()
                    } else {
                        String::new()
                    }
                }}

            </span>
            <span class="absolute top-0 right-0 bottom-0 py-2 px-3">
                <button
                    on:click=close
                    class="flex justify-center items-center rounded-full transition-colors duration-150 hover:bg-red-400 active:scale-95 size-6"
                    aria-label=move || t_string!(i18n, messages.chat.close_alert).to_string()
                >
                    x
                </button>
            </span>
        </div>
    }
}
