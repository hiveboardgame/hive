use crate::{
    components::layouts::base_layout::BaseLayout,
    pages::{
        account::Account, analysis::Analysis, challenge_view::ChallengeView, home::Home,
        login::Login, play::Play, profile_view::ProfileView, register::Register, user_get::UserGet,
    },
    providers::{
        alerts::provide_alerts, auth_context::provide_auth, challenges::provide_challenges,
        color_scheme::provide_color_scheme, game_state::provide_game_state, games::provide_games,
        navigation_controller::provide_navigation_controller, online_users::provide_users,
        ping::provide_ping, refocus::provide_refocus, timer::provide_timer,
        web_socket::provide_websocket,
    },
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

pub const LEPTOS_OUTPUT_NAME: &str = std::env!("LEPTOS_OUTPUT_NAME");

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
    provide_ping();
    provide_navigation_controller();
    let url = "/ws/";
    provide_websocket(url);
    provide_auth();
    provide_alerts();
    provide_refocus();

    view! {
        <Stylesheet id="leptos" href=format!("/pkg/{}.css", LEPTOS_OUTPUT_NAME)/>

        <meta name="viewport" content="width=device-width, initial-scale=1"/>

        // content for this welcome page
        <Router>
            <Routes>
                <Route
                    path=""
                    view=|| {
                        view! {
                            <BaseLayout>
                                <Outlet/>
                            </BaseLayout>
                        }
                    }
                >

                    <Route path="" ssr=SsrMode::InOrder view=|| view! { <Home/> }/>
                    <Route path="/@/:username" view=|| view! { <ProfileView/> }/>
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
