use crate::components::atoms::rating::icon_for_speed;
use crate::i18n::*;
use leptos::*;
use leptos_icons::*;
use shared_types::{GameSpeed, TimeInfo, TimeMode};

#[component]
pub fn TimeRow(
    time_info: MaybeSignal<TimeInfo>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let i18n = use_i18n();
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
            TimeMode::Untimed => "No time limit".to_owned().into_view(),
            TimeMode::RealTime => format!(
                "{} + {}",
                time_info.base.expect("Time exists") / 60,
                time_info.increment.expect("Increment exists"),
            )
            .into_view(),

            TimeMode::Correspondence => {
                if let Some(base) = time_info.base {
                    t!(
                        i18n,
                        game.time_mode.correspondence.days_side,
                        count = move || (base / 86400)
                    )
                    .into_view()
                } else if let Some(increment) = time_info.increment {
                    t!(
                        i18n,
                        game.time_mode.correspondence.days_move,
                        count = move || (increment / 86400)
                    )
                    .into_view()
                } else {
                    "".into_view()
                }
            }
        }
    };
    view! {
        <div class="flex gap-1 justify-start items-center">
            {icon} <p class=extend_tw_classes>{text}</p>
        </div>
    }
}
