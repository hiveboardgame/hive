use crate::{
    components::{
        layouts::base_layout::BaseLayout,
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
        resources::Resources,
        rules::Rules,
        rules_summary::RulesSummary,
        strategy::Strategy,
        top_players::TopPlayers,
        tournament::Tournament,
        tournament_create::TournamentCreate,
        tournaments::{HostingTournaments, JoinedTournaments, Tournaments, TournamentsByStatus},
        tutorial::Tutorial,
    },
    providers::{
        challenges::provide_challenges,
        chat::provide_chat,
        games::provide_games,
        online_users::provide_users,
        provide_alerts,
        provide_api_requests,
        provide_auth,
        provide_challenge_params,
        provide_config,
        provide_direct_challenge,
        provide_notifications,
        provide_ping,
        provide_referer,
        provide_server_updates,
        provide_sounds,
        refocus::provide_refocus,
        schedules::provide_schedules,
        websocket::provide_websocket,
        AuthContext,
    },
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
use shared_types::{GameProgress, GameThread, TournamentStatus};

// 1 year in milliseconds
const LOCALE_MAX_AGE: i64 = 1000 * 60 * 60 * 24 * 365;

fn auth_pending_fallback() -> impl IntoView {
    view! { <div aria-hidden="true"></div> }
}

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
    let direct_challenge = provide_direct_challenge();
    let auth = expect_context::<AuthContext>();
    let is_logged_in = move || auth.logged_in.get();
    let is_admin = move || auth.admin.get();
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
                        <Route path=path!("/login") view=|| view! { <Login /> } />
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
                                view=|| {
                                    view! { <MessagesGameThread thread=GameThread::Players /> }
                                }
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
                            fallback=auth_pending_fallback
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
                            fallback=auth_pending_fallback
                            view=|| view! { <Config /> }
                        />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/notifications")
                            redirect_path=|| "/login"
                            fallback=auth_pending_fallback
                            view=|| view! { <Notifications /> }
                        />
                        <Route path=path!("/tournament/:nanoid") view=|| view! { <Tournament /> } />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/tournaments/create")
                            redirect_path=|| "/login"
                            fallback=auth_pending_fallback
                            view=|| view! { <TournamentCreate /> }
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
                                        <TournamentsByStatus status=TournamentStatus::NotStarted />
                                    }
                                }
                            />
                            <Route
                                path=path!("future")
                                view=|| {
                                    view! {
                                        <TournamentsByStatus status=TournamentStatus::NotStarted />
                                    }
                                }
                            />
                            <Route
                                path=path!("inprogress")
                                view=|| {
                                    view! {
                                        <TournamentsByStatus status=TournamentStatus::InProgress />
                                    }
                                }
                            />
                            <Route
                                path=path!("finished")
                                view=|| {
                                    view! {
                                        <TournamentsByStatus status=TournamentStatus::Finished />
                                    }
                                }
                            />
                            <ProtectedRoute
                                condition=is_logged_in
                                path=path!("joined")
                                redirect_path=|| "/login"
                                fallback=auth_pending_fallback
                                view=|| view! { <JoinedTournaments /> }
                            />
                            <ProtectedRoute
                                condition=is_logged_in
                                path=path!("hosting")
                                redirect_path=|| "/login"
                                fallback=auth_pending_fallback
                                view=|| view! { <HostingTournaments /> }
                            />
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
                            fallback=auth_pending_fallback
                            view=|| view! { <Admin /> }
                        />
                        <ProtectedRoute
                            condition=is_admin
                            path=path!("/admin/telemetry")
                            redirect_path=|| "/"
                            fallback=auth_pending_fallback
                            view=|| view! { <AdminTelemetry /> }
                        />
                        <ProtectedRoute
                            condition=is_admin
                            path=path!("/admin/push-metrics")
                            redirect_path=|| "/"
                            fallback=auth_pending_fallback
                            view=|| view! { <AdminPushMetrics /> }
                        />
                    </ParentRoute>
                </Routes>
            </Router>
            <DirectChallengeModal state=direct_challenge />
        </I18nContextProvider>
    }
}
