use crate::{
    components::layouts::base_layout::BaseLayout,
    i18n::I18nContextProvider,
    pages::{
        account::Account,
        admin::Admin,
        analysis::Analysis,
        challenge_view::ChallengeView,
        config::Config,
        donate::Donate,
        faq::Faq,
        home::Home,
        login::Login,
        play::Play,
        profile_view::{DisplayGames, ProfileView},
        puzzles::Puzzles,
        register::Register,
        resources::Resources,
        rules::Rules,
        rules_summary::RulesSummary,
        strategy::Strategy,
        top_players::TopPlayers,
        tournament::Tournament,
        tournament_create::TournamentCreate,
        tournaments::Tournaments,
        tutorial::Tutorial,
    },
    providers::{
        challenges::provide_challenges, chat::provide_chat, game_state::provide_game_state,
        games::provide_games, games_search::provide_profile_games,
        navigation_controller::provide_navigation_controller, online_users::provide_users,
        provide_alerts, provide_auth, provide_challenge_params, provide_config,
        provide_notifications, provide_ping, provide_sounds, refocus::provide_refocus,
        schedules::provide_schedules, timer::provide_timer,
        tournament_ready::provide_tournament_ready, tournaments::provide_tournaments,
        user_search::provide_user_search, websocket::provide_websocket,
    },
};
use leptos::*;
use leptos_i18n::context::CookieOptions;
use leptos_meta::*;
use leptos_router::*;
use leptos_use::SameSite;
use shared_types::GameProgress;

// 1 year in milliseconds
const LOCALE_MAX_AGE: i64 = 1000 * 60 * 60 * 24 * 365;

#[component]
pub fn App() -> impl IntoView {
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
    provide_challenge_params();
    provide_alerts();
    provide_refocus();
    provide_chat();
    provide_user_search();
    provide_tournaments();
    provide_notifications();
    provide_tournament_ready();
    provide_schedules();
    provide_sounds();
    provide_profile_games();
    view! {
        <I18nContextProvider cookie_options=CookieOptions::default()
            .max_age(LOCALE_MAX_AGE)
            .same_site(SameSite::Lax)>
            <Stylesheet id="leptos" href="/pkg/HiveGame.css" />
            <Router trailing_slash=TrailingSlash::Redirect>
                <Routes>
                    <Route
                        path=""
                        view=|| {
                            view! {
                                <BaseLayout>
                                    <Outlet />
                                </BaseLayout>
                            }
                        }
                    >

                        <Route path="" ssr=SsrMode::InOrder view=|| view! { <Home /> } />
                        <Route
                            path="/@/:username"
                            view=|| {
                                view! {
                                    <ProfileView>
                                        <Outlet />
                                    </ProfileView>
                                }
                            }
                        >

                            <Route
                                path=""
                                view=|| view! { <DisplayGames tab_view=GameProgress::Playing /> }
                            />
                            <Route
                                path="playing"
                                view=|| view! { <DisplayGames tab_view=GameProgress::Playing /> }
                            />
                            <Route
                                path="finished"
                                view=|| view! { <DisplayGames tab_view=GameProgress::Finished /> }
                            />
                            <Route
                                path="unstarted"
                                view=|| view! { <DisplayGames tab_view=GameProgress::Unstarted /> }
                            />
                        </Route>
                        <Route path="/register" view=|| view! { <Register /> } />
                        <Route path="/top_players" view=|| view! { <TopPlayers /> } />
                        <Route path="/login" view=|| view! { <Login /> } />
                        <Route path="/account" view=|| view! { <Account /> } />
                        <Route path="/challenge/:nanoid" view=|| view! { <ChallengeView /> } />
                        <Route path="/analysis" view=|| view! { <Analysis /> } />
                        <Route path="/config" view=|| view! { <Config /> } />
                        <Route path="/tournament/:nanoid" view=|| view! { <Tournament /> } />
                        <Route path="/tournaments/create" view=|| view! { <TournamentCreate /> } />
                        <Route path="/tournaments" view=|| view! { <Tournaments /> } />
                        <Route path="/donate" view=|| view! { <Donate /> } />
                        <Route path="/faq" view=|| view! { <Faq /> } />
                        <Route path="/puzzles" view=|| view! { <Puzzles /> } />
                        <Route path="/rules" view=|| view! { <Rules /> } />
                        <Route path="/strategy" view=|| view! { <Strategy /> } />
                        <Route path="/resources" view=|| view! { <Resources /> } />
                        <Route path="/tutorial" view=|| view! { <Tutorial /> } />
                        <Route path="/rules_summary" view=|| view! { <RulesSummary /> } />
                        <Route
                            path="/game/:nanoid"
                            ssr=SsrMode::PartiallyBlocked
                            view=|| view! { <Play /> }
                        />
                        <Route path="/admin" view=|| view! { <Admin /> } />
                    </Route>
                </Routes>
            </Router>
        </I18nContextProvider>
    }
}
