use leptos::prelude::*;
use leptos_use::{use_interval_fn_with_options, utils::Pausable, UseIntervalFnOptions};

use crate::providers::{AlertType, AlertsContext};

#[component]
pub fn Alert() -> impl IntoView {
    let alerts = expect_context::<AlertsContext>();
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
    let color_and_text = move || match (alerts.last_alert)() {
        Some(AlertType::Error(_)) => ("bg-red-100 border-ladybug-red", "Error! "),
        Some(AlertType::Warn(_)) => ("bg-yellow-100 border-orange-twilight", "Warning! "),
        Some(AlertType::Notification(_)) => ("bg-blue-100 border-pillbug-teal", "Notification: "),
        None => ("", ""),
    };

    create_effect(move |_| {
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
                    color_and_text().0,
                    if (alerts.last_alert)().is_some() { "block" } else { "hidden" },
                )
            }

            role="alert"
        >
            <strong class="font-bold">{move || color_and_text().1}</strong>
            <span class="block">
                {move || {
                    if let Some(alert) = (alerts.last_alert)() {
                        alert.to_string()
                    } else {
                        String::new()
                    }
                }}

            </span>
            <span class="absolute top-0 right-0 bottom-0 px-3 py-2 border-y-2">
                <button
                    on:click=close
                    class="flex justify-center items-center w-6 h-6 rounded-full duration-300 hover:bg-red-400 active:scale-95"
                    aria-label="Close"
                >
                    x
                </button>
            </span>
        </div>
    }
}
