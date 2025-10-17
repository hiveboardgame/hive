use futures::channel::mpsc;
use futures::stream::{AbortHandle, Abortable};

use crate::websocket::new_style::client::{client_handler, ClientApi};
use crate::{
    components::{layouts::base_layout::BaseLayout, organisms::display_games::DisplayGames},
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
        profile_view::ProfileView,
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
        challenges::provide_challenges, chat::provide_chat, games::provide_games,
        online_users::provide_users, provide_alerts, provide_api_requests, provide_auth,
        provide_challenge_params, provide_config, provide_notifications, provide_ping,
        provide_referer, provide_server_updates, provide_sounds, refocus::provide_refocus,
        schedules::provide_schedules, websocket::provide_websocket, AuthContext,
    },
};
use leptos::prelude::*;
use leptos_i18n::context::CookieOptions;
use leptos_meta::*;
use leptos_router::{
    components::{Outlet, ParentRoute, ProtectedRoute, Route, Router, Routes},
    path,
};
use leptos_use::SameSite;
use shared_types::{GameProgress, TournamentStatus};

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

    //expects websocket, client_api

    provide_context(ClientApi::default());
    provide_auth();

    //expects auth
    provide_games();

    //expects auth, challengeStateSignal, websocket
    provide_api_requests();

    //expects auth, api_requests, gameStateSignal
    provide_chat();

    // we'll only listen for websocket messages on the client
    if cfg!(feature = "hydrate") {
        let client_api = expect_context::<ClientApi>();
        let ws_restart = client_api.signal_restart_ws();
        let task_handle = StoredValue::<Option<AbortHandle>>::new(None);
        // Start or restart the ws task
        Effect::watch(
            ws_restart,
            move |_, _, _| {
                // If a task is already running, cancel it
                if let Some(handle) = task_handle.get_value() {
                    handle.abort();
                }
                // Create a new abortable future
                let (abort_handle, abort_reg) = AbortHandle::new_pair();

                // Store the handle so we can stop it later
                task_handle.set_value(Some(abort_handle));

                let (tx, rx) = mpsc::channel(1);
                client_api.set_sender(tx);
                leptos::task::spawn_local(async move {
                    // Make it abortable
                    let _ = Abortable::new(client_handler(rx), abort_reg).await;
                    leptos::logging::log!("Task stopped or aborted");
                });
            },
            false,
        );
    }
    let auth = expect_context::<AuthContext>();
    let is_logged_in = move || auth.user.with(|a| a.is_some()).into();
    let is_admin = move || Some(auth.user.with(|a| a.as_ref().is_some_and(|v| v.user.admin)));
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
                        <Route path=path!("/tournament/:nanoid") view=|| view! { <Tournament /> } />
                        <ProtectedRoute
                            condition=is_logged_in
                            path=path!("/tournaments/create")
                            redirect_path=|| "/login"
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
                                view=|| view! { <JoinedTournaments /> }
                            />
                            <ProtectedRoute
                                condition=is_logged_in
                                path=path!("hosting")
                                redirect_path=|| "/login"
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
                            view=|| view! { <Admin /> }
                        />
                    </ParentRoute>
                </Routes>
            </Router>
        </I18nContextProvider>
    }
}
