use crate::providers::game_state::GameStateSignal;
use lazy_static::lazy_static;
use leptos::*;
use leptos_meta::Title;
use leptos_router::RouterContext;
use regex::Regex;
lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[component]
pub fn NextGameButton() -> impl IntoView {
    let navigate = leptos_router::use_navigate();
    let game_state_signal = expect_context::<GameStateSignal>();

    let next_games = move || {
        let mut games = game_state_signal.signal.get().next_games;
        let router = expect_context::<RouterContext>();
        if let Some(caps) = NANOID.captures(&router.pathname().get_untracked()) {
            let nanoid = caps.name("nanoid").map_or("", |m| m.as_str());
            games.retain(|game| *game != nanoid);
        }
        games
    };

    let color = move || match next_games().len() {
        0 => "hidden",
        _ => "bg-red-700 duration-300 hover:bg-red-600 text-white rounded-md px-2 py-1 m-2",
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
        let mut games = next_games();
        if let Some(game) = games.pop() {
            // TODO: this needs to happen when a move has successfully been played
            //game_state_signal.set_next_games(games);
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
