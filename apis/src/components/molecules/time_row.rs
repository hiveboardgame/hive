use leptos::*;
use leptos_icons::{
    AiIcon::AiMailOutlined,
    BiIcon::{BiInfiniteRegular, BiStopwatchRegular},
    Icon,
};
use shared_types::time_mode::TimeMode;

#[component]
pub fn TimeRow(
    time_mode: TimeMode,
    time_base: Option<i32>,
    increment: Option<i32>,
) -> impl IntoView {
    view! {
        <p class="flex items-center gap-1">
            {match time_mode {
                TimeMode::Untimed => {
                    view! {
                        <Icon icon=Icon::from(BiInfiniteRegular)/>
                        <p>"No time limit "</p>
                    }
                }
                TimeMode::RealTime => {
                    view! {
                        <Icon icon=Icon::from(BiStopwatchRegular)/>
                        {format!(
                            "{}m + {}s",
                            time_base.expect("Time exists") / 60,
                            increment.expect("Increment exists"),
                        )}
                    }
                }
                TimeMode::Correspondence if time_base.is_some() => {
                    view! {
                        <Icon icon=Icon::from(AiMailOutlined)/>
                        <p>{format!("{} days/side", time_base.expect("Time exists") / 86400)}</p>
                    }
                }
                TimeMode::Correspondence if increment.is_some() => {
                    view! {
                        <Icon icon=Icon::from(AiMailOutlined)/>
                        <p>{format!("{} days/move ", increment.expect("Time exists") / 86400)}</p>
                    }
                }
                _ => unreachable!(),
            }}

        </p>
    }
}
