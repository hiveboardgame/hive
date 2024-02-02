use leptos::*;
use leptos_icons::*;
use shared_types::time_mode::TimeMode;

#[component]
pub fn TimeRow(
    time_mode: TimeMode,
    time_base: Option<i32>,
    increment: Option<i32>,
    #[prop(optional)] extend_tw_classes: &'static str,
) -> impl IntoView {
    let time_mode = store_value(time_mode);
    let icon = move || match time_mode() {
        TimeMode::Untimed => icondata::BiInfiniteRegular,
        TimeMode::RealTime => icondata::BiStopwatchRegular,

        TimeMode::Correspondence if time_base.is_some() => icondata::AiMailOutlined,

        TimeMode::Correspondence if increment.is_some() => icondata::AiMailOutlined,

        _ => unreachable!(),
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
            <Icon icon=icon() class="w-4 h-4"/>
            <p class=extend_tw_classes>{text}</p>
        </div>
    }
}
