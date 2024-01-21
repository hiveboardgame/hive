use crate::providers::{
    game_state::GameStateSignal, games::GamesSignal,
    navigation_controller::NavigationControllerSignal,
};
use leptos::{logging::log, *};
use leptos_meta::Title;

#[component]
pub fn NextGameButton() -> impl IntoView {
    let navigate = leptos_router::use_navigate();
    let navigation_controller = expect_context::<NavigationControllerSignal>();
    let next_games = move || {
        let mut games = expect_context::<GamesSignal>();
        games.update_next_games();
        let mut next_games = games.signal.get().next_games;
        log!("all next games: {:?}", next_games);
        if let Some(nanoid) = navigation_controller.signal.get().nanoid {
            log!("nanoid: {:?}", nanoid);
            next_games.retain(|g| *g != nanoid);
        }
        log!("next games without current: {:?}", next_games);
        next_games
    };
    let color = move || {
        match next_games().len() {
            0 => "hidden",
            _ => "bg-red-700 transform transition-transform duration-300 active:scale-95 hover:bg-red-600 text-white rounded-md px-2 py-1 m-2",
        }
    };
    let title_text = move || match next_games().len() {
        0 => String::from("HiveGame.com"),
        i => format!("({}) HiveGame.com", i),
    };
    let text = move || match next_games().len() {
        0 => String::new(),
        1 => String::from("Next"),
        i => format!("Next ({})", i),
    };
    let onclick = move |_| {
        let next_games = next_games();
        if let Some(game) = next_games.first() {
            let mut game_state = expect_context::<GameStateSignal>();
            game_state.full_reset();
            let mut games = expect_context::<GamesSignal>();
            games.visit_game(game.to_owned());
            navigate(&format!("/game/{}", game), Default::default());
        } else {
            navigate("/", Default::default());
        }
    };

    view! {
        <Title text=title_text/>

        <div class="relative">
            <button on:click=onclick class=color>
                {text}
            </button>
        </div>
    }
}
