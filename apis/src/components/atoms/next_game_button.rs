use crate::providers::{games::GamesSignal, navigation_controller::NavigationControllerSignal};
use leptos::prelude::*;
use leptos_icons::Icon;
use shared_types::TimeMode;

#[component]
pub fn NextGameButton(time_mode: StoredValue<TimeMode>) -> impl IntoView {
    let navigate = leptos_router::hooks::use_navigate();
    let navigation_controller = expect_context::<NavigationControllerSignal>();
    let mut games = expect_context::<GamesSignal>();
    let next_games = move || {
        let game_id = navigation_controller
            .game_signal
            .get()
            .game_id
            .unwrap_or_default();
        match time_mode.get_value() {
            TimeMode::Untimed => games.own.get().next_untimed,
            TimeMode::RealTime => games.own.get().next_realtime,
            TimeMode::Correspondence => games.own.get().next_correspondence,
        }
        .iter()
        .filter(|gp| gp.game_id != game_id)
        .count()
    };
    let icon = move || match time_mode.get_value() {
        TimeMode::Untimed => icondata::BiInfiniteRegular,
        TimeMode::RealTime => icondata::BiStopwatchRegular,
        TimeMode::Correspondence => icondata::AiMailOutlined,
    };
    let style = move || {
        match next_games() {
            0 => "hidden",
            _ => "flex place-items-center bg-ladybug-red transform transition-transform duration-300 active:scale-95 hover:bg-red-400 text-white rounded-md px-2 py-1 m-1",
        }
    };
    let text = move || format!(": {}", next_games());
    let onclick = move |_| {
        if let Some(game) = games.visit(time_mode.get_value()) {
            navigate(&format!("/game/{}", game), Default::default());
        } else {
            navigate("/", Default::default());
        }
    };

    view! {
        <button on:click=onclick class=style>
            <Icon icon=icon() attr:class="w-4 h-4" />
            {text}
        </button>
    }
}
