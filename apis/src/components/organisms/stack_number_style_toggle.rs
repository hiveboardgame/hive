use crate::{common::StackNumberStyle, i18n::*, providers::Config};
use leptos::prelude::*;

#[component]
pub fn StackNumberStyleToggle() -> impl IntoView {
    view! {
        <div class="ui-choice-group">
            <StackNumberStyleButton style=StackNumberStyle::Circles />
            <StackNumberStyleButton style=StackNumberStyle::Hexes />
        </div>
    }
}

#[component]
fn StackNumberStyleButton(style: StackNumberStyle) -> impl IntoView {
    let i18n = use_i18n();
    let Config(config, set_cookie) = expect_context();
    let is_active = move || config.with(|config| config.stack_number_style == style);

    view! {
        <button
            class="ui-choice ui-choice-md"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
            on:click=move |_| {
                set_cookie
                    .update(|cookie| {
                        if let Some(cookie) = cookie {
                            cookie.stack_number_style = style;
                        }
                    });
            }
        >
            {move || match style {
                StackNumberStyle::Circles => t_string!(i18n, user_config.stack_number_style_circles),
                StackNumberStyle::Hexes => t_string!(i18n, user_config.stack_number_style_hexes),
            }}
        </button>
    }
}
