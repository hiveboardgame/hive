use crate::components::atoms::rating::icon_for_speed;
use crate::i18n::*;
use leptos::{either::EitherOf3, prelude::*};
use leptos_icons::*;
use shared_types::{GameSpeed, TimeInfo, TimeMode};

#[component]
pub fn TimeRow(
    #[prop(into)] time_info: Signal<TimeInfo>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let i18n = use_i18n();
    let time_mode = Signal::derive(move || time_info.get_untracked().mode);
    let icon = move || {
        let time_info = time_info();
        let speed = match time_mode() {
            TimeMode::Untimed => GameSpeed::Untimed,
            TimeMode::Correspondence => GameSpeed::Correspondence,
            TimeMode::RealTime => {
                GameSpeed::from_base_increment(time_info.base, time_info.increment)
            }
        };
        view! { <Icon icon=icon_for_speed(&speed) attr:class="w-4 h-4" /> }
    };
    let text = move || {
        let time_info = time_info();
        match time_mode() {
            TimeMode::Untimed => EitherOf3::A("No time limit".to_owned()),
            TimeMode::RealTime => EitherOf3::A(format!(
                "{} + {}",
                time_info.base.expect("Time exists") / 60,
                time_info.increment.expect("Increment exists"),
            )),

            TimeMode::Correspondence => {
                if let Some(base) = time_info.base {
                    EitherOf3::B(t!(
                        i18n,
                        game.time_mode.correspondence.days_side,
                        count = move || (base / 86400)
                    ))
                } else if let Some(increment) = time_info.increment {
                    EitherOf3::C(t!(
                        i18n,
                        game.time_mode.correspondence.days_move,
                        count = move || (increment / 86400)
                    ))
                } else {
                    EitherOf3::A("".to_string())
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
