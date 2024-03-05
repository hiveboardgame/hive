use crate::{
    components::layouts::base_layout::BaseLayout,
    pages::{
        account::Account,
        analysis::Analysis,
        challenge_view::ChallengeView,
        config::Config,
        display_games::DisplayGames,
        home::Home,
        login::Login,
        play::Play,
        profile_view::{ProfileGamesView, ProfileView},
        register::Register,
        top_players::TopPlayers,
    },
    providers::{
        alerts::provide_alerts, auth_context::provide_auth, challenges::provide_challenges,
        color_scheme::provide_color_scheme, config::config::provide_config,
        game_state::provide_game_state, games::provide_games,
        navigation_controller::provide_navigation_controller, online_users::provide_users,
        ping::provide_ping, refocus::provide_refocus, timer::provide_timer,
        websocket::context::provide_websocket,
    },
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

pub const LEPTOS_OUTPUT_NAME: &str = std::env!("LEPTOS_OUTPUT_NAME");

#[component]
pub fn App() -> impl IntoView {
    provide_color_scheme();
    provide_config();
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
                    <Route
                        path="/@/:username"
                        view=|| {
                            view! {
                                <ProfileView>
                                    <Outlet/>
                                </ProfileView>
                            }
                        }
                    >

                        <Route
                            path=""
                            view=|| view! { <DisplayGames tab_view=ProfileGamesView::Playing/> }
                        />
                        <Route
                            path="playing"
                            view=|| view! { <DisplayGames tab_view=ProfileGamesView::Playing/> }
                        />
                        <Route
                            path="finished"
                            view=|| view! { <DisplayGames tab_view=ProfileGamesView::Finished/> }
                        />
                    </Route>
                    <Route path="/register" view=|| view! { <Register/> }/>
                    <Route path="/top_players" view=|| view! { <TopPlayers/> }/>
                    <Route path="/login" view=|| view! { <Login/> }/>
                    <Route path="/account" view=|| view! { <Account/> }/>
                    <Route path="/challenge/:nanoid" view=|| view! { <ChallengeView/> }/>
                    <Route path="/analysis" view=|| view! { <Analysis/> }/>
                    <Route path="/config" view=|| view! { <Config/> }/>
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
