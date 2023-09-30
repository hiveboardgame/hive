use crate::pages::{
    home::Home, play::PlayPage, sign_in::SignIn, sign_up::SignUp, user_account::UserAccount,
    ws::WsPage, logout::LogOut,
};
use common::game_state::GameStateSignal;
use common::web_socket::provide_websocket;
use leptos::logging::log;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

pub mod common;
pub mod components;
pub mod error_template;
pub mod functions;
pub mod pages;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    log!("Setting up game state");
    provide_context(create_rw_signal(GameStateSignal::new()));

    let url = "ws://0.0.0.0:3000/ws/67e55044-10b1-426f-9247-bb680e5fe0c8";
    provide_websocket(url);

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>

        <meta name="viewport" content="width=device-width, initial-scale=1"/>
        // sets the document title
        <Title text="Welcome to Hive"/>

        // content for this welcome page
        <Router>
            <main class="h-screen w-screen overflow-hidden">
                <Routes>
                    <Route path="" view=|| view! { <Home/> }/>
                    <Route path="/play" view=|| view! { <PlayPage extend_tw_classes="h-full w-full"/> }/>
                    <Route path="/sign_up" view=|| view! { <SignUp/>}/>
                    <Route path="/sign_in" view=|| view! { <SignIn/>}/>
                    <Route path="/logout" view=|| view! { <LogOut/>}/>
                    <Route path="/hws" view=|| view! { <WsPage/> }/>
                    <Route path="/user_account" view=|| view! { <UserAccount/> }/>
                </Routes>
            </main>
        </Router>
    }
}
