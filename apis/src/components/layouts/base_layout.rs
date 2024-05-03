use crate::components::atoms::title::Title;
use crate::components::atoms::og::OG;
use crate::components::molecules::alert::Alert;
use crate::components::organisms::header::Header;
use crate::providers::api_requests::ApiRequests;
use crate::providers::auth_context::AuthContext;
use crate::providers::color_scheme::ColorScheme;
use crate::providers::navigation_controller::NavigationControllerSignal;
use crate::providers::ping::PingSignal;
use crate::providers::refocus::RefocusSignal;
use crate::providers::websocket::context::WebsocketContext;
use chrono::Utc;
use lazy_static::lazy_static;
use leptos::logging::log;
use leptos::*;
use leptos_meta::*;
use leptos_router::use_location;
use leptos_use::core::ConnectionReadyState;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn, use_window_focus};
use regex::Regex;

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}
pub const COMMON_LINK_STYLE: &str = "bg-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded";
pub const DROPDOWN_BUTTON_STYLE: &str= "h-full p-2 hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 whitespace-nowrap block";

#[component]
pub fn BaseLayout(children: Children) -> impl IntoView {
    let color_scheme = expect_context::<ColorScheme>();
    let color_scheme_meta = move || {
        if (color_scheme.prefers_dark)() {
            "dark".to_string()
        } else {
            "light".to_string()
        }
    };
    let auth_context = expect_context::<AuthContext>();
    let ws = expect_context::<WebsocketContext>();

    create_effect(move |_| {
        let location = use_location();
        let mut navi = expect_context::<NavigationControllerSignal>();
        let pathname = (location.pathname)();
        let nanoid = if let Some(caps) = NANOID.captures(&pathname) {
            caps.name("nanoid").map(|m| m.as_str().to_string())
        } else {
            None
        };
        if (auth_context.user)().is_some() && (ws.ready_state)() == ConnectionReadyState::Open {
            navi.update_nanoid(nanoid);
        };
    });

    let api = ApiRequests::new();
    let ping = expect_context::<PingSignal>();
    let ws = expect_context::<WebsocketContext>();

    let focused = use_window_focus();
    let _ = watch(
        focused,
        move |focused, _, _| {
            let mut refocus = expect_context::<RefocusSignal>();
            log!("Focus changed");
            if *focused {
                refocus.refocus();
            } else {
                refocus.unfocus();
            }
        },
        false,
    );

    let counter = RwSignal::new(0_u64);
    let retry_at = RwSignal::new(2_u64);
    let Pausable { .. } = use_interval_fn(
        move || {
            api.ping();
            counter.update(|c| *c += 1);
            match ws.ready_state.get() {
                ConnectionReadyState::Closed => {
                    if retry_at.get() == counter.get() {
                        log!("Reconnecting due to ReadyState");
                        ws.open();
                        counter.update(|c| *c = 0);
                        counter.update(|r| *r *= 2);
                    }
                }
                ConnectionReadyState::Open => {
                    counter.update(|c| *c = 0);
                }
                _ => {}
            }
            if Utc::now()
                .signed_duration_since(ping.signal.get_untracked().last_update)
                .num_seconds()
                >= 5
                && retry_at.get() == counter.get()
            {
                log!("Reconnecting due to ping duration");
                ws.open();
                counter.update(|c| *c = 0);
                counter.update(|r| *r *= 2);
            };
        },
        1000,
    );

    view! {
        <Title/>
        <OG/>
        <Meta name="color-scheme" content=color_scheme_meta/>
        <Meta
            name="viewport"
            content="width=device-width, initial-scale=1, interactive-widget=resizes-content, virtual-keyboard=resize-layout"
        />
        <Html class=move || {
            match (color_scheme.prefers_dark)() {
                true => "dark",
                false => "",
            }
        }/>

        <Body/>
        <main class="min-h-screen w-full bg-light dark:bg-dark text-xs sm:text-sm md:text-md touch-manipulations">
            <Header/>
            <Alert/>
            {children()}
        </main>
    }
}
