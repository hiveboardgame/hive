use crate::i18n::*;
use crate::providers::Config;
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

    // Auto-update background color when switching themes (if using default colors)
    Effect::watch(
        move || config().prefers_dark,
        move |is_dark_mode, prev_dark_mode, _| {
            // Only update if theme actually changed (not on initial load)
            if let Some(prev) = prev_dark_mode {
                if *is_dark_mode != *prev {
                    set_cookie.update(|c| {
                        if let Some(cookie) = c {
                            // Only update if using default colors, not custom colors
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
        <div class="mb-4">
            <p class="m-1 text-black dark:text-white">{t!(i18n, user_config.background_color)}</p>

            <div class="flex gap-2 items-center">
                <input
                    type="color"
                    prop:value=current_color
                    on:input=handle_color_change
                    class="w-16 h-10 bg-transparent rounded border cursor-pointer"
                    title=move || t_string!(i18n, user_config.background_color)
                />

                // Reset button - only show when using custom color
                <Show when=is_using_custom>
                    <button
                        on:click=reset_to_default
                        class="px-3 py-2 text-sm bg-gray-200 rounded transition-colors duration-200 dark:bg-gray-700 hover:bg-gray-300 dark:hover:bg-gray-600"
                        title="Reset to default"
                    >
                        "Reset"
                    </button>
                </Show>
            </div>

            // Show current color value for reference
            <div class="mt-1 text-xs text-gray-600 dark:text-gray-400">
                {move || format!("Current: {}", current_color())}
            </div>
        </div>
    }
}
