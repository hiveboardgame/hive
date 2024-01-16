use crate::{
    components::layouts::base_layout::BaseLayout,
    pages::{
        account::Account, analysis::Analysis, challenge_view::ChallengeView, home::Home,
        login::Login, play::Play, players::PlayersView, profile_view::ProfileView,
        register::Register, user_get::UserGet,
    },
    providers::{
        alerts::provide_alerts, auth_context::provide_auth, challenges::provide_challenges,
        color_scheme::provide_color_scheme, game_state::provide_game_state, games::provide_games,
        navigation_controller::provide_navigation_controller, online_users::provide_users,
        timer::provide_timer, web_socket::provide_websocket,
    },
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
    provide_challenges();
    provide_games();
    provide_users();
    provide_timer();
    provide_navigation_controller();
    let url = "/ws/";
    provide_websocket(url);
    provide_auth();
    provide_alerts();

    view! {
        <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>

        <meta name="viewport" content="width=device-width, initial-scale=1"/>

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

                    <Route path="" ssr=SsrMode::InOrder view=|| view! { <Home/> }/>
                    <Route path="/@/:username" view=|| view! { <ProfileView/> }/>
                    <Route path="/players" view=|| view! { <PlayersView/> }/>
                    <Route path="/register" view=|| view! { <Register/> }/>
                    <Route path="/login" view=|| view! { <Login/> }/>
                    <Route path="/account" view=|| view! { <Account/> }/>
                    <Route path="/get_user" view=|| view! { <UserGet/> }/>
                    <Route path="/challenge/:nanoid" view=|| view! { <ChallengeView/> }/>
                    <Route path="/analysis" view=|| view! { <Analysis/> }/>
                    <Route
                        path="/game/:nanoid"
                        ssr=SsrMode::PartiallyBlocked
                        view=|| view! { <Play/> }
                    />
                </Route>
            </Routes>
        </Router>
    }
}
