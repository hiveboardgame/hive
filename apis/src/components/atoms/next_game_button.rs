use crate::providers::games::GamesSignal;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::hooks::use_params_map;
use shared_types::{GameId, TimeMode};

#[component]
pub fn NextGameButton(time_mode: StoredValue<TimeMode>) -> impl IntoView {
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let games = expect_context::<GamesSignal>();
    let next_games = move || {
        let game_id = game_id();
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
    let next_game_id = move || {
        match time_mode.get_value() {
            TimeMode::Untimed => games.own.get().next_untimed,
            TimeMode::RealTime => games.own.get().next_realtime,
            TimeMode::Correspondence => games.own.get().next_correspondence,
        }.peek().map(|gp| format!("/game/{}", gp.game_id)).unwrap_or_default()
    };

    view! {
        <a class=style href=next_game_id>
            <Icon icon=icon() attr:class="w-4 h-4" />
            {text}
        </a>
    }
}
