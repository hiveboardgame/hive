#[cfg(not(feature = "ssr"))]
use crate::components::layouts::navigation_focus::current_browser_location_key;
use crate::{
    components::{
        layouts::{
            base_layout::BaseLayout,
            navigation_focus::{NavigationFocus, NavigationFocusState},
        },
        organisms::{direct_challenge_modal::DirectChallengeModal, display_games::DisplayGames},
    },
    i18n::I18nContextProvider,
    pages::{
        account::Account,
        admin::Admin,
        admin_push_metrics::AdminPushMetrics,
        admin_telemetry::AdminTelemetry,
        analysis::Analysis,
        challenge_view::ChallengeView,
        config::Config,
        donate::Donate,
        faq::Faq,
        forgot_password::ForgotPassword,
        game_search::GameSearch,
        home::Home,
        login::Login,
        messages::{
            MessagesDmThread,
            MessagesGameThread,
            MessagesGlobalThread,
            MessagesIndex,
            MessagesLayout,
            MessagesTournamentThread,
        },
        notifications::Notifications,
        play::Play,
        profile_view::{ProfileMe, ProfileView},
        puzzles::Puzzles,
        register::Register,
        reset_password::ResetPassword,
        resources::Resources,
        rules::Rules,
        rules_summary::RulesSummary,
        strategy::Strategy,
        top_players::{TopBots, TopPlayers},
        tournament::TournamentRoutes,
        tournament_create::{TournamentCreate, TournamentCreateChooser, TournamentCreationKind},
        tournaments::{MineTournamentsRedirect, TournamentList, Tournaments},
        tutorial::Tutorial,
    },
    providers::{
        challenges::provide_challenges,
        chat::provide_chat,
        games::provide_games,
        login_redirect_url,
        online_users::provide_users,
        provide_active_tournament_state,
        provide_alerts,
        provide_api_requests,
        provide_auth,
        provide_challenge_params,
        provide_config,
        provide_direct_challenge,
        provide_game_state,
        provide_notifications,
        provide_ping,
        provide_referer,
        provide_server_updates,
        provide_sounds,
        refocus::provide_refocus,
        schedules::provide_schedules,
        websocket::provide_websocket,
        AuthContext,
        AuthIdentity,
    },
    responses::TournamentCategory,
};
use leptos::prelude::*;
use leptos_i18n::context::CookieOptions;
use leptos_meta::*;
use leptos_router::{
    components::{
        Outlet,
        ParentRoute,
        ProtectedParentRoute,
        ProtectedRoute,
        Route,
        Router,
        Routes,
    },
    path,
};
use leptos_use::SameSite;
use shared_types::{GameProgress, GameThread};

#[cfg(not(feature = "ssr"))]
use leptos::{ev, leptos_dom::helpers::window_event_listener};

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
    provide_active_tournament_state();
    provide_refocus();
    provide_alerts();
    provide_challenge_params();
    provide_config();
    provide_users();
    provide_challenges();
    provide_game_state();
    provide_websocket("/ws/");

    //expects websocket
    provide_auth();

    //expects auth
    provide_games();

    //expects auth, challengeStateSignal, websocket
    provide_api_requests();

    //expects auth and websocket
    provide_chat();
    let direct_challenge = provide_direct_challenge();
    let auth = expect_context::<AuthContext>();
    let is_logged_in = move || {
        auth.identity
            .get()
            .map(|identity| matches!(identity, AuthIdentity::User(_)))
    };
    let is_admin = move || auth.admin.get();
    #[cfg(not(feature = "ssr"))]
    let initial_location = current_browser_location_key();
    #[cfg(feature = "ssr")]
    let initial_location = None;
    let navigation_focus = NavigationFocusState::new(initial_location);
    provide_context(navigation_focus.clone());
    #[cfg(not(feature = "ssr"))]
    {
        let navigation_focus = navigation_focus.clone();
        let popstate_handle = window_event_listener(ev::popstate, move |_| {
            navigation_focus.begin_history_navigation(current_browser_location_key());
        });
        on_cleanup(move || popstate_handle.remove());
    }
    view! {
        <I18nContextProvider cookie_options=CookieOptions::default()
            .max_age(LOCALE_MAX_AGE)
            .same_site(SameSite::Lax)
            .path("/")>
            <Router>
                <NavigationFocus state=navigation_focus />
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

                        <Route path=path!("") view=|| view! { <Home /> } />
                        <Route path=path!("/@/me") view=|| view! { <ProfileMe /> } />
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
                        <Route path=path!("/archive") view=|| view! { <GameSearch /> } />
                        <Route path=path!("/top_players") view=|| view! { <TopPlayers /> } />
                        <Route path=path!("/top_bots") view=|| view! { <TopBots /> } />
                        <Route path=path!("/login") view=|| view! { <Login /> } />
                        <Route
                            path=path!("/forgot-password")
                            view=|| view! { <ForgotPassword /> }
                        />
                        <Route path=path!("/reset-password") view=|| view! { <ResetPassword /> } />
                        <ProtectedParentRoute
                            condition=is_logged_in
                            path=path!("/message")
                            redirect_path=|| "/login"
                            view=MessagesLayout
                        >
                            <Route path=path!("") view=MessagesIndex />
                            <Route path=path!("global") view=MessagesGlobalThread />
                            <Route path=path!("dm/:username") view=MessagesDmThread />
                            <Route path=path!("tournament/:nanoid") view=MessagesTournamentThread />
                            <Route
                                path=path!("game/:nanoid/players")
                                view=|| view! { <MessagesGameThread thread=GameThread::Players /> }
                            />
                            <Route
                                path=path!("game/:nanoid/spectators")
                                view=|| {
                                    view! { <MessagesGameThread thread=GameThread::Spectators /> }
                                }
                            />
                        </ProtectedParentRoute>
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/account")
                            redirect_path=|| "/login"
                            view=|| view! { <Account /> }
                        />
                        <Route
                            path=path!("/challenge/:nanoid")
                            view=|| view! { <ChallengeView /> }
                        />
                        <Route path=path!("/analysis") view=|| view! { <Analysis /> } />
                        <Route path=path!("/analysis/:nanoid") view=|| view! { <Analysis /> } />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/config")
                            redirect_path=|| "/login"
                            view=|| view! { <Config /> }
                        />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/notifications")
                            redirect_path=|| "/login"
                            view=|| view! { <Notifications /> }
                        />
                        <TournamentRoutes />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/tournaments/create")
                            redirect_path=login_redirect_url
                            view=|| view! { <TournamentCreateChooser /> }
                        />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/tournaments/create/arena")
                            redirect_path=login_redirect_url
                            view=|| {
                                view! { <TournamentCreate kind=TournamentCreationKind::Arena /> }
                            }
                        />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/tournaments/create/swiss")
                            redirect_path=login_redirect_url
                            view=|| {
                                view! { <TournamentCreate kind=TournamentCreationKind::Swiss /> }
                            }
                        />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/tournaments/create/round-robin")
                            redirect_path=login_redirect_url
                            view=|| {
                                view! {
                                    <TournamentCreate kind=TournamentCreationKind::RoundRobin />
                                }
                            }
                        />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/tournaments/create/elimination")
                            redirect_path=login_redirect_url
                            view=|| {
                                view! {
                                    <TournamentCreate kind=TournamentCreationKind::Elimination />
                                }
                            }
                        />
                        <ParentRoute
                            path=path!("/tournaments")
                            view=|| {
                                view! {
                                    <Tournaments>
                                        <Outlet />
                                    </Tournaments>
                                }
                            }
                        >
                            <Route
                                path=path!("")
                                view=|| {
                                    view! {
                                        <TournamentList category=TournamentCategory::Upcoming />
                                    }
                                }
                            />
                            <Route
                                path=path!("in-progress")
                                view=|| {
                                    view! {
                                        <TournamentList category=TournamentCategory::InProgress />
                                    }
                                }
                            />
                            <Route
                                path=path!("finished")
                                view=|| {
                                    view! {
                                        <TournamentList category=TournamentCategory::Finished />
                                    }
                                }
                            />
                            <ProtectedParentRoute
                                condition=is_logged_in
                                path=path!("mine")
                                redirect_path=|| "/login"
                                view=|| view! { <Outlet /> }
                            >
                                <Route path=path!("") view=MineTournamentsRedirect />
                                <Route
                                    path=path!("joined")
                                    view=|| {
                                        view! {
                                            <TournamentList category=TournamentCategory::Joined />
                                        }
                                    }
                                />
                                <Route
                                    path=path!("organizing")
                                    view=|| {
                                        view! {
                                            <TournamentList category=TournamentCategory::Organizing />
                                        }
                                    }
                                />
                                <Route
                                    path=path!("invitations")
                                    view=|| {
                                        view! {
                                            <TournamentList category=TournamentCategory::Invitations />
                                        }
                                    }
                                />
                                <Route
                                    path=path!("history")
                                    view=|| {
                                        view! {
                                            <TournamentList category=TournamentCategory::History />
                                        }
                                    }
                                />
                            </ProtectedParentRoute>
                        </ParentRoute>
                        <Route path=path!("/donate") view=|| view! { <Donate /> } />
                        <Route path=path!("/faq") view=|| view! { <Faq /> } />
                        <Route path=path!("/puzzles") view=|| view! { <Puzzles /> } />
                        <Route path=path!("/rules") view=|| view! { <Rules /> } />
                        <Route path=path!("/strategy") view=|| view! { <Strategy /> } />
                        <Route path=path!("/resources") view=|| view! { <Resources /> } />
                        <Route path=path!("/tutorial") view=|| view! { <Tutorial /> } />
                        <Route path=path!("/rules_summary") view=|| view! { <RulesSummary /> } />
                        <Route path=path!("/game/:nanoid") view=|| view! { <Play /> } />
                        <ProtectedRoute
                            condition=is_admin
                            path=path!("/admin")
                            redirect_path=|| "/"
                            view=|| view! { <Admin /> }
                        />
                        <ProtectedRoute
                            condition=is_admin
                            path=path!("/admin/telemetry")
                            redirect_path=|| "/"
                            view=|| view! { <AdminTelemetry /> }
                        />
                        <ProtectedRoute
                            condition=is_admin
                            path=path!("/admin/push-metrics")
                            redirect_path=|| "/"
                            view=|| view! { <AdminPushMetrics /> }
                        />
                    </ParentRoute>
                </Routes>
            </Router>
            <DirectChallengeModal state=direct_challenge />
        </I18nContextProvider>
    }
}
