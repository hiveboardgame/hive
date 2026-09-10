use crate::{components::atoms::rating::icon_for_speed, i18n::*};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{
    clock::{Clock, CorrespondenceClock},
    GameSpeed,
};

pub fn format_compact_duration(seconds: u32) -> String {
    if seconds == 0 {
        String::from("0s")
    } else if seconds.is_multiple_of(3600) {
        format!("{}h", seconds / 3600)
    } else if seconds.is_multiple_of(60) {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

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
    let text = move || match time_control() {
        None => t_string!(i18n, game.time_mode.untimed).to_string(),
        Some(Clock::Realtime(clock)) => format!(
            "{} + {}",
            format_compact_duration(clock.base_seconds.get()),
            format_compact_duration(clock.increment_seconds),
        ),
        Some(Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each })) => {
            if seconds_each.get().is_multiple_of(86400) {
                let days = i32::try_from(seconds_each.get() / 86400)
                    .expect("u32 seconds contain an i32 number of days");
                t_string!(i18n, game.time_mode.correspondence.days_side, count = days).to_string()
            } else {
                t_string!(
                    i18n,
                    game.time_mode.correspondence.duration_side,
                    duration = format_compact_duration(seconds_each.get())
                )
                .to_string()
            }
        }
        Some(Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move })) => {
            if seconds_per_move.get().is_multiple_of(86400) {
                let days = i32::try_from(seconds_per_move.get() / 86400)
                    .expect("u32 seconds contain an i32 number of days");
                t_string!(i18n, game.time_mode.correspondence.days_move, count = days).to_string()
            } else {
                t_string!(
                    i18n,
                    game.time_mode.correspondence.duration_move,
                    duration = format_compact_duration(seconds_per_move.get())
                )
                .to_string()
            }
        }
    };
    view! {
        <div class="flex gap-1 justify-start items-center">
            {icon} <p class=extend_tw_classes>{text}</p>
        </div>
    }
}
