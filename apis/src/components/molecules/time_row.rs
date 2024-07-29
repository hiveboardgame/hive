use crate::components::atoms::rating::icon_for_speed;
use leptos::*;
use leptos_icons::*;
use shared_types::{GameSpeed, TimeInfo, TimeMode};

#[component]
pub fn TimeRow(
    time_info: MaybeSignal<TimeInfo>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let time_mode = store_value(time_info.get_untracked().mode);
    let icon = move || {
        let time_info = time_info();
        let speed = match time_mode() {
            TimeMode::Untimed => GameSpeed::Untimed,
            TimeMode::Correspondence => GameSpeed::Correspondence,
            TimeMode::RealTime => {
                GameSpeed::from_base_increment(time_info.base, time_info.increment)
            }
        };
        view! { <Icon icon=icon_for_speed(&speed) class="w-4 h-4"/> }
    };
    let text = move || {
        let time_info = time_info();
        match time_mode() {
            TimeMode::Untimed => "No time limit".to_owned(),
            TimeMode::RealTime => format!(
                "{}m + {}s",
                time_info.base.expect("Time exists") / 60,
                time_info.increment.expect("Increment exists"),
            ),

            TimeMode::Correspondence if time_info.base.is_some() => {
                format!("{} days/side", time_info.base.expect("Time exists") / 86400)
            }

            TimeMode::Correspondence if time_info.increment.is_some() => {
                format!(
                    "{} days/move ",
                    time_info.increment.expect("Time exists") / 86400
                )
            }

            _ => unreachable!(),
        }
    };
    view! {
        <div class="flex gap-1 justify-start items-center">
            {icon} <p class=extend_tw_classes>{text}</p>
        </div>
    }
}
