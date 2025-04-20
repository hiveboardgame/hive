use crate::{
    components::layouts::base_layout::BaseLayout,
    i18n::I18nContextProvider,
    pages::{
        account::Account,
        admin::Admin,
        analysis::Analysis,
        bevy::BevyExamplePage,
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
        challenges::provide_challenges, chat::provide_chat, games::provide_games,
        online_users::provide_users, provide_alerts, provide_api_requests, provide_auth,
        provide_challenge_params, provide_config, provide_notifications, provide_ping,
        provide_referer, provide_server_updates, provide_sounds, refocus::provide_refocus,
        schedules::provide_schedules, websocket::provide_websocket,
    },
};
use leptos::prelude::*;
use leptos_i18n::context::CookieOptions;
use leptos_meta::*;
use leptos_router::{
    components::{Outlet, ParentRoute, Route, Router, Routes},
    path, SsrMode,
};
use leptos_use::SameSite;
use shared_types::GameProgress;

// 1 year in milliseconds
const LOCALE_MAX_AGE: i64 = 1000 * 60 * 60 * 24 * 365;

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    //These dont expect any other context, can be provided in any order
    provide_ping();
    provide_referer();
    provide_server_updates();
    provide_schedules();
    provide_notifications();
    provide_sounds();
    provide_refocus();
    provide_alerts();
    provide_challenge_params();
    provide_config();
    provide_users();
    provide_challenges();
    provide_websocket("/ws/");

    //expects websocket
    provide_auth();

    //expects auth
    provide_games();

    //expects auth, challengeStateSignal, websocket
    provide_api_requests();

    //expects auth, api_requests, gameStateSignal
    provide_chat();
    view! {
        <I18nContextProvider cookie_options=CookieOptions::default()
            .max_age(LOCALE_MAX_AGE)
            .same_site(SameSite::Lax)
            .path("/")>
            <Router>
                <Routes fallback=|| "404 Not Found">
                    <ParentRoute
                        path=path!("")
                        view=|| {
                            view! {
                                <BaseLayout>
                                    <Outlet />
                                </BaseLayout>
                            }
                        }
                    >

                        <Route path=path!("") ssr=SsrMode::InOrder view=|| view! { <Home /> } />
                        <ParentRoute
                            path=path!("/@/:username")
                            view=|| {
                                view! {
                                    <ProfileView>
                                        <Outlet />
                                    </ProfileView>
                                }
                            }
                        >

                            <Route
                                path=path!("")
                                view=|| view! { <DisplayGames tab_view=GameProgress::Playing /> }
                            />
                            <Route
                                path=path!("playing")
                                view=|| view! { <DisplayGames tab_view=GameProgress::Playing /> }
                            />
                            <Route
                                path=path!("finished")
                                view=|| view! { <DisplayGames tab_view=GameProgress::Finished /> }
                            />
                            <Route
                                path=path!("unstarted")
                                view=|| view! { <DisplayGames tab_view=GameProgress::Unstarted /> }
                            />
                        </ParentRoute>
                        <Route path=path!("/register") view=|| view! { <Register /> } />
                        <Route path=path!("/top_players") view=|| view! { <TopPlayers /> } />
                        <Route path=path!("/login") view=|| view! { <Login /> } />
                        <Route path=path!("/account") view=|| view! { <Account /> } />
                        <Route
                            path=path!("/challenge/:nanoid")
                            view=|| view! { <ChallengeView /> }
                        />
                        <Route path=path!("/analysis") view=|| view! { <Analysis /> } />
                        <Route path=path!("/config") view=|| view! { <Config /> } />
                        <Route path=path!("/tournament/:nanoid") view=|| view! { <Tournament /> } />
                        <Route
                            path=path!("/tournaments/create")
                            view=|| view! { <TournamentCreate /> }
                        />
                        <Route path=path!("/tournaments") view=|| view! { <Tournaments /> } />
                        <Route path=path!("/donate") view=|| view! { <Donate /> } />
                        <Route path=path!("/faq") view=|| view! { <Faq /> } />
                        <Route path=path!("/puzzles") view=|| view! { <Puzzles /> } />
                        <Route path=path!("/rules") view=|| view! { <Rules /> } />
                        <Route path=path!("/strategy") view=|| view! { <Strategy /> } />
                        <Route path=path!("/resources") view=|| view! { <Resources /> } />
                        <Route path=path!("/tutorial") view=|| view! { <Tutorial /> } />
                        <Route path=path!("/rules_summary") view=|| view! { <RulesSummary /> } />
                        <Route
                            path=path!("/game/:nanoid")
                            ssr=SsrMode::PartiallyBlocked
                            view=|| view! { <Play /> }
                        />
                        <Route path=path!("/admin") view=|| view! { <Admin /> } />
                        <Route path=path!("/bevy") view=|| view! { <BevyExamplePage /> } />
                    </ParentRoute>
                </Routes>
            </Router>
        </I18nContextProvider>
    }
}
