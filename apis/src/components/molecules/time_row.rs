use crate::{common::format_time_control, components::atoms::rating::icon_for_speed, i18n::*};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{Clock, GameSpeed};

#[component]
pub fn TimeRow(
    /// `None` means a known untimed control. Callers with no uniform or loaded
    /// control must gate the row instead of passing that absence here.
    #[prop(into)]
    time_control: Signal<Option<Clock>>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let i18n = use_i18n();
    let icon = move || {
        let speed = time_control().map_or(GameSpeed::Untimed, GameSpeed::from);
        view! { <Icon icon=icon_for_speed(speed) attr:class="size-4" /> }
    };
    let text = move || format_time_control(i18n, time_control());
    view! {
        <div class="flex gap-1 justify-start items-center">
            {icon} <p class=extend_tw_classes>{text}</p>
        </div>
    }
}
