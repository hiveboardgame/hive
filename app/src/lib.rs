use common::game_state::GameStateSignal;
use common::web_socket::provide_websocket;
use leptos::logging::log;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

pub mod atoms;
pub mod common;
pub mod error_template;
#[cfg(feature = "ssr")]
pub mod functions;
pub mod molecules;
pub mod organisms;
pub mod pages;
use crate::pages::{home::Home, play::PlayPage, ws::WsPage, user_create::UserCreate, user_get::UserGet};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    log!("Setting up game state");
    provide_context(create_rw_signal(GameStateSignal::new()));

    let url = "ws://127.0.0.1:3000/ws/67e55044-10b1-426f-9247-bb680e5fe0c8";
    provide_websocket(url);

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/start-actix-workspace.css"/>

        <meta name="viewport" content="width=device-width, initial-scale=1"/>
        // sets the document title
        <Title text="Welcome to Hive"/>

        // content for this welcome page
        <Router>
            <main class="h-screen w-screen overflow-hidden">
                <Routes>
                    <Route path="" view=|| view! { <Home/> }/>
                    <Route path="/play" view=|| view! { <PlayPage/> }/>
                    <Route path="/hws" view=|| view! { <WsPage/> }/>
                    <Route path="/user" view=|| view! { <UserCreate/> }/>
                    <Route path="/user_get" view=|| view! { <UserGet/> }/>
                </Routes>
            </main>
        </Router>
    }
}
