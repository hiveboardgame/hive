use crate::providers::{
    game_state::GameStateSignal, games::GamesSignal,
    navigation_controller::NavigationControllerSignal,
};
use leptos::*;
use leptos_meta::Title;

#[component]
pub fn NextGameButton() -> impl IntoView {
    let navigate = leptos_router::use_navigate();
    let navigation_controller = expect_context::<NavigationControllerSignal>();
    let games = expect_context::<GamesSignal>();
    let next_games = move || {
        games.signal.with(|games_state| {
            if let Some(nanoid) = navigation_controller.signal.get().nanoid {
                games_state
                    .next_games
                    .iter()
                    .filter(|g| **g != nanoid)
                    .cloned()
                    .collect()
            } else {
                games_state.next_games.clone()
            }
        })
    };
    let style = move || {
        match next_games().len() {
            0 => "hidden",
            _ => "bg-ladybug-red transform transition-transform duration-300 active:scale-95 hover:bg-red-400 text-white rounded-md px-2 py-1 m-1",
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
        let mut games = expect_context::<GamesSignal>();
        if let Some(game) = games.visit_game() {
            let mut game_state = expect_context::<GameStateSignal>();
            game_state.full_reset();
            navigate(&format!("/game/{}", game), Default::default());
        } else {
            navigate("/", Default::default());
        }
    };

    view! {
        <Title text=title_text/>

        <div class="relative">
            <button on:click=onclick class=style>
                {text}
            </button>
        </div>
    }
}
