use crate::{
    components::layouts::base_layout::BaseLayout,
    pages::{
        account::Account,
        analysis::Analysis,
        challenge_view::ChallengeView,
        config::Config,
        display_games::DisplayGames,
        donate::Donate,
        faq::Faq,
        home::Home,
        login::Login,
        play::Play,
        profile_view::{ProfileGamesView, ProfileView},
        puzzles::Puzzles,
        register::Register,
        resources::Resources,
        rules::Rules,
        strategy::Strategy,
        top_players::TopPlayers,
        tournaments::Tournaments,
        tutorial::Tutorial,
    },
    providers::{
        provide_alerts, provide_auth, challenges::provide_challenges,
        chat::provide_chat, provide_color_scheme, provide_config,
        game_state::provide_game_state, games::provide_games,
        navigation_controller::provide_navigation_controller, online_users::provide_users,
        provide_ping, refocus::provide_refocus, timer::provide_timer,
        websocket::provide_websocket,
    },
};
use leptos::*;
use leptos_meta::*;
use leptos_router::*;

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
    provide_chat();

    view! {
        <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>
        <Router trailing_slash=TrailingSlash::Redirect>
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
                    <Route path="/tournaments" view=|| view! { <Tournaments/> }/>
                    <Route path="/donate" view=|| view! { <Donate/> }/>
                    <Route path="/faq" view=|| view! { <Faq/> }/>
                    <Route path="/puzzles" view=|| view! { <Puzzles/> }/>
                    <Route path="/rules" view=|| view! { <Rules/> }/>
                    <Route path="/strategy" view=|| view! { <Strategy/> }/>
                    <Route path="/resources" view=|| view! { <Resources/> }/>
                    <Route path="/tutorial" view=|| view! { <Tutorial/> }/>
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
