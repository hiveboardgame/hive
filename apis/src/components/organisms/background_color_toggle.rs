use crate::{i18n::*, providers::Config};
use leptos::prelude::*;

#[component]
pub fn BackgroundColorToggle() -> impl IntoView {
    let i18n = use_i18n();
    let Config(config, set_cookie) = expect_context();

    let current_color = Signal::derive(move || {
        config.with(|c| c.tile.get_effective_background_color(c.prefers_dark))
    });
    let is_using_custom =
        Signal::derive(move || config.with(|c| c.tile.is_using_custom_background(c.prefers_dark)));

    let handle_color_change = move |ev| {
        let value = event_target_value(&ev);
        set_cookie.update(|c| {
            if let Some(cookie) = c {
                cookie.tile.background_color = Some(value);
            }
        });
    };

    let reset_to_default = move |_| {
        set_cookie.update(|c| {
            if let Some(cookie) = c {
                cookie.tile.background_color = None;
            }
        });
    };

    Effect::watch(
        move || config().prefers_dark,
        move |is_dark_mode, prev_dark_mode, _| {
            if let Some(prev) = prev_dark_mode {
                if *is_dark_mode != *prev {
                    set_cookie.update(|c| {
                        if let Some(cookie) = c {
                            if !cookie.tile.is_using_custom_background(*is_dark_mode) {
                                cookie.tile.background_color = None;
                            }
                        }
                    });
                }
            }
        },
        false,
    );

    view! {
        <div class="flex flex-col gap-2">
            <p class="ui-field-label">{t!(i18n, user_config.background_color)}</p>

            <div class="flex gap-2 items-center">
                <input
                    type="color"
                    prop:value=current_color
                    on:input=handle_color_change
                    class="p-1 w-16 h-10 bg-transparent rounded-lg border shadow-sm cursor-pointer border-black/10 dark:border-white/10"
                    title=move || t_string!(i18n, user_config.background_color)
                />

                <Show when=is_using_custom>
                    <button
                        on:click=reset_to_default
                        class="ui-button ui-button-secondary ui-button-sm"
                        title="Reset to default"
                    >
                        "Reset"
                    </button>
                </Show>
            </div>

            <div class="ui-field-helper">{move || format!("Current: {}", current_color())}</div>
        </div>
    }
}
