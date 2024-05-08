use crate::components::atoms::rating::icon_for_speed;
use leptos::*;
use leptos_icons::*;
use shared_types::{game_speed::GameSpeed, time_mode::TimeMode};

#[component]
pub fn TimeRow(
    time_mode: TimeMode,
    time_base: Option<i32>,
    increment: Option<i32>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let time_mode = store_value(time_mode);
    let icon = move || {
        let speed = match time_mode() {
            TimeMode::Untimed => GameSpeed::Untimed,
            TimeMode::Correspondence => GameSpeed::Correspondence,
            TimeMode::RealTime => GameSpeed::from_base_increment(time_base, increment),
        };
        view! { <Icon icon=icon_for_speed(&speed) class="w-4 h-4"/> }
    };
    let text = move || match time_mode() {
        TimeMode::Untimed => "No time limit".to_owned(),
        TimeMode::RealTime => format!(
            "{}m + {}s",
            time_base.expect("Time exists") / 60,
            increment.expect("Increment exists"),
        ),

        TimeMode::Correspondence if time_base.is_some() => {
            format!("{} days/side", time_base.expect("Time exists") / 86400)
        }

        TimeMode::Correspondence if increment.is_some() => {
            format!("{} days/move ", increment.expect("Time exists") / 86400)
        }

        _ => unreachable!(),
    };
    view! {
        <div class="flex items-center gap-1 justify-start">
            {icon}
            <p class=extend_tw_classes>{text}</p>
        </div>
    }
}
