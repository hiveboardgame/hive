use crate::providers::games::GamesSignal;
use leptos::prelude::*;
use leptos_icons::Icon;
use leptos_router::hooks::use_params_map;
use shared_types::{GameId, TimeMode};

#[component]
pub fn NextGameButton(time_mode: TimeMode, mut games: GamesSignal) -> impl IntoView {
    let time_mode = StoredValue::new(time_mode);
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let next_games = move || {
        let game_id = game_id();
        games.own.with(|own| {
            let games_list = match time_mode.get_value() {
                TimeMode::Untimed => &own.next_untimed,
                TimeMode::RealTime => &own.next_realtime,
                TimeMode::Correspondence => &own.next_correspondence,
            };
            games_list.iter().filter(|gp| gp.game_id != game_id).count()
        })
    };
    let icon = move || match time_mode.get_value() {
        TimeMode::Untimed => icondata_bi::BiInfiniteRegular,
        TimeMode::RealTime => icondata_bi::BiStopwatchRegular,
        TimeMode::Correspondence => icondata_ai::AiMailOutlined,
    };
    let style = move || {
        match next_games() {
            0 => "hidden",
            _ => "no-link-style flex place-items-center bg-ladybug-red transform transition-transform duration-300 active:scale-95 hover:bg-red-400 text-white rounded-md px-2 py-1 m-1",
        }
    };
    let text = move || format!(": {}", next_games());
    let game_id = move || {
        games.own.with(|own| {
            let games_list = match time_mode.get_value() {
                TimeMode::Untimed => &own.next_untimed,
                TimeMode::RealTime => &own.next_realtime,
                TimeMode::Correspondence => &own.next_correspondence,
            };
            games_list.peek().map(|gp| gp.game_id.clone())
        })
    };

    let href_game_id = move || game_id().map(|id| format!("/game/{id}"));

    let onclick = move |_| {
        if let Some(game_id) = game_id() {
            games.visit(time_mode.get_value(), game_id);
        };
    };

    view! {
        <a class=style href=href_game_id on:click=onclick>
            <Icon icon=icon() attr:class="size-4" />
            {text}
        </a>
    }
}
