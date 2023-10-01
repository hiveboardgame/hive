use crate::components::layouts::base_layout::BaseLayout;
use crate::pages::{
    home::Home, logout::LogOut, play::PlayPage, sign_in::SignIn, sign_up::SignUp,
    user_account::UserAccount, ws::WsPage,
};
use common::game_state::GameStateSignal;
use common::web_socket::provide_websocket;
use leptos::logging::log;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use providers::color_scheme::*;

pub mod common;
pub mod components;
pub mod error_template;
pub mod functions;
pub mod pages;
pub mod providers;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    _ = provide_color_scheme();
    provide_meta_context();
    log!("Setting up game state");
    provide_context(create_rw_signal(GameStateSignal::new()));

    let url = "ws://0.0.0.0:3000/ws/67e55044-10b1-426f-9247-bb680e5fe0c8";
    _ = provide_websocket(url);

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>

        <meta name="viewport" content="width=device-width, initial-scale=1"/>
        // sets the document title
        <Title text="Welcome to Hive"/>

        // content for this welcome page
        <Router>
                <Routes>
                <Route
                    path=""
                    view=|| {
                        view! {
                            <BaseLayout>
                                //<ErrorBoundary fallback=| errors| {
                                //    view! {  <ErrorTemplate errors=errors/> }
                                //}>
                                    <Outlet/>
                                //</ErrorBoundary>
                            </BaseLayout>
                        }
                    }
                >
                    <Route path="" view=|| view! { <Home/> }/>
                    <Route path="/play" view=|| view! { <PlayPage/> }/>
                    <Route path="/sign_up" view=|| view! { <SignUp/>}/>
                    <Route path="/sign_in" view=|| view! { <SignIn/>}/>
                    <Route path="/logout" view=|| view! { <LogOut/>}/>
                    <Route path="/hws" view=|| view! { <WsPage/> }/>
                    <Route path="/user_account" view=|| view! { <UserAccount/> }/>
                    </Route>
                </Routes>
        </Router>
    }
}
