use crate::components::layouts::base_layout::BaseLayout;
use crate::pages::{
    account::Account, challenge_create::ChallengeCreate, challenge_view::ChallengeView, home::Home,
    login::Login, play::Play, register::Register, user_get::UserGet, ws::WsPage,
};

use leptos::logging::log;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use crate::providers::{
    auth_context::provide_auth, color_scheme::provide_color_scheme, game_state::provide_game_state,
    web_socket::provide_websocket,
};

#[component]
pub fn App() -> impl IntoView {
    provide_auth();
    provide_color_scheme();
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    log!("Setting up game state");
    provide_game_state();
    let url = "ws://127.0.0.1:3000/ws/67e55044-10b1-426f-9247-bb680e5fe0c8";
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
                    <Route path="/register" view=|| view! { <Register/>}/>
                    <Route path="/login" view=|| view! { <Login/>}/>
                    <Route path="/hws" view=|| view! { <WsPage/> }/>
                    <Route path="/account" view=|| view! { <Account/> }/>
                    <Route path="/get_user" view=|| view! { <UserGet/> }/>
                    <Route path="/challenge/:nanoid" view=|| view! { <ChallengeView/> }/>
                    <Route path="/challenges/create" view=|| view! { <ChallengeCreate/> }/>
                    <Route path="/play/:nanoid" view=|| view! { <Play/> }/>
                    </Route>
                </Routes>
        </Router>
    }
}
