use crate::{
    components::layouts::base_layout::BaseLayout,
    pages::{
        account::Account, challenge_view::ChallengeView, home::Home, login::Login, play::Play,
        players::PlayersView, profile_view::ProfileView, register::Register, user_get::UserGet,
        ws::WsPage,
    },
    providers::{
        auth_context::provide_auth, color_scheme::provide_color_scheme,
        game_controller::provide_game_controller, game_state::provide_game_state,
        web_socket::provide_websocket,
    },
};
use lazy_static::lazy_static;
use leptos::*;
use leptos_meta::*;
use leptos_router::*;
use regex::Regex;

#[component]
pub fn App() -> impl IntoView {
    provide_color_scheme();
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();
    provide_game_state();
    provide_game_controller();
    let url = "/ws/";
    provide_websocket(url);
    provide_auth();

    lazy_static! {
        static ref NANOID: Regex =
            Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
    }

    // create_effect(move |_| {
    //     let router = expect_context::<RouterContext>();
    //     let mut game_controller = expect_context::<GameControllerSignal>();
    //     if let Some(caps) = NANOID.captures(&(router.pathname())()) {
    //         if let Some(m) = caps.name("nanoid") {
    //             let nanoid = m.as_str();
    //             game_controller.join(nanoid.to_string());
    //         }
    //     }
    // });

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

                    <Route path="" ssr=SsrMode::InOrder view=|| view! { <Home/> }/>
                    <Route path="/@/:username" view=|| view! { <ProfileView/> }/>
                    <Route path="/players" view=|| view! { <PlayersView/> }/>
                    <Route path="/register" view=|| view! { <Register/> }/>
                    <Route path="/login" view=|| view! { <Login/> }/>
                    <Route path="/hws" view=|| view! { <WsPage/> }/>
                    <Route path="/account" view=|| view! { <Account/> }/>
                    <Route path="/get_user" view=|| view! { <UserGet/> }/>
                    <Route path="/challenge/:nanoid" view=|| view! { <ChallengeView/> }/>
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
