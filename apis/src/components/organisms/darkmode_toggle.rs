use crate::providers::Config;
use leptos::prelude::*;
use leptos_icons::Icon;

#[component]
pub fn DarkModeToggle(variant: DarkModeToggleVariant) -> impl IntoView {
    let Config(config, set_cookie) = expect_context();
    let wrapper_class = match variant {
        DarkModeToggleVariant::Button => "inline-flex items-center",
        DarkModeToggleVariant::Header => "inline-flex h-full justify-center items-center",
        DarkModeToggleVariant::Dropdown => "",
    };
    let button_class = match variant {
        DarkModeToggleVariant::Button => "ui-button ui-button-secondary ui-button-sm",
        DarkModeToggleVariant::Header => "ui-header-icon-button",
        DarkModeToggleVariant::Dropdown => "ui-dropdown-link",
    };
    let has_label = move || {
        matches!(
            variant,
            DarkModeToggleVariant::Button | DarkModeToggleVariant::Dropdown
        )
    };
    let mode_label = move || {
        if config().prefers_dark {
            "Light mode"
        } else {
            "Dark mode"
        }
    };
    let mode_value = move || {
        if config().prefers_dark {
            "dark"
        } else {
            "light"
        }
    };
    let mode_icon = move || {
        let (icon, class) = if config().prefers_dark {
            (icondata_bs::BsSunFill, "size-4 text-orange-twilight")
        } else {
            (
                icondata_bs::BsMoonStarsFill,
                "size-4 text-gray-800 dark:text-gray-100",
            )
        };

        view! { <Icon icon attr:class=class /> }
    };
    let toggle_dark_mode = move |_| {
        set_cookie.update(|c| {
            if let Some(cookie) = c {
                cookie.prefers_dark = !cookie.prefers_dark;
            }
        });
    };

    view! {
        <div class=wrapper_class>
            <button
                type="button"
                on:click=toggle_dark_mode
                class=button_class
                value=mode_value
                title=mode_label
                aria-label=mode_label
            >
                <Show when=has_label>
                    <span>{mode_label}</span>
                </Show>
                {mode_icon}
            </button>
        </div>
    }
}

#[derive(Clone, Copy)]
pub enum DarkModeToggleVariant {
    Button,
    Header,
    Dropdown,
}
