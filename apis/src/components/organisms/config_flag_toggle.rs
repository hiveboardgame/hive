use crate::{
    i18n::*,
    providers::{config::ConfigOpts, Config},
};
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigFlag {
    Preselect,
    Premove,
    DragAndDrop,
}

impl ConfigFlag {
    fn get(self, config: &ConfigOpts) -> bool {
        match self {
            Self::Preselect => config.allow_preselect,
            Self::Premove => config.allow_premove,
            Self::DragAndDrop => config.drag_and_drop,
        }
    }

    fn set(self, config: &mut ConfigOpts, enabled: bool) {
        match self {
            Self::Preselect => config.allow_preselect = enabled,
            Self::Premove => config.allow_premove = enabled,
            Self::DragAndDrop => config.drag_and_drop = enabled,
        }
    }
}

#[component]
pub fn ConfigFlagToggle(flag: ConfigFlag) -> impl IntoView {
    view! {
        <div class="ui-choice-group">
            <ConfigFlagButton flag enabled=true />
            <ConfigFlagButton flag enabled=false />
        </div>
    }
}

#[component]
fn ConfigFlagButton(flag: ConfigFlag, enabled: bool) -> impl IntoView {
    let i18n = use_i18n();
    let Config(config, set_cookie) = expect_context();
    let is_active = move || config.with(|c| flag.get(c)) == enabled;

    view! {
        <button
            class="ui-choice ui-choice-md"
            class:ui-choice-active=is_active
            class:ui-choice-inactive=move || !is_active()
            on:click=move |_| {
                set_cookie
                    .update(|c| {
                        if let Some(cookie) = c {
                            flag.set(cookie, enabled);
                        }
                    });
            }
        >
            {if enabled {
                t!(i18n, user_config.toggle_buttons.yes).into_any()
            } else {
                t!(i18n, user_config.toggle_buttons.no).into_any()
            }}
        </button>
    }
}
