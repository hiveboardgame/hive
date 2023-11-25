use crate::components::layouts::base_layout::BaseLayout;
use crate::pages::{
    account::Account, challenge_view::ChallengeView, home::Home, login::Login, play::Play,
    profile_view::ProfileView, register::Register, user_get::UserGet, ws::WsPage,
};

use crate::providers::{
    auth_context::provide_auth, color_scheme::provide_color_scheme, game_state::provide_game_state,
    web_socket::provide_websocket,
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    provide_color_scheme();
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    provide_game_state();
    let url = "/ws/";
    _ = provide_websocket(url);
    provide_auth();

    view! {
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
                                // <ErrorBoundary fallback=| errors| {
                                // view! {  <ErrorTemplate errors=errors/> }
                                // }>
                                <Outlet/>
                            // </ErrorBoundary>
                            </BaseLayout>
                        }
                    }
                >

                    <Route path="" view=|| view! { <Home/> }/>
                    <Route path="/@/:username" view=|| view! { <ProfileView/> }/>
                    <Route path="/register" view=|| view! { <Register/> }/>
                    <Route path="/login" view=|| view! { <Login/> }/>
                    <Route path="/hws" view=|| view! { <WsPage/> }/>
                    <Route path="/account" view=|| view! { <Account/> }/>
                    <Route path="/get_user" view=|| view! { <UserGet/> }/>
                    <Route path="/challenge/:nanoid" view=|| view! { <ChallengeView/> }/>
                    <Route path="/game/:nanoid" view=|| view! { <Play/> }/>
                </Route>
            </Routes>
        </Router>
    }
}

