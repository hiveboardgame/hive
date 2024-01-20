use crate::components::molecules::alert::Alert;
use crate::components::organisms::header::Header;
use crate::providers::api_requests::ApiRequests;
use crate::providers::color_scheme::ColorScheme;
use crate::providers::navigation_controller::NavigationControllerSignal;
use crate::providers::ping::PingSignal;
use crate::providers::web_socket::WebsocketContext;
use chrono::Utc;
use lazy_static::lazy_static;
use leptos::logging::log;
use leptos::*;
use leptos_meta::*;
use leptos_router::use_location;
use leptos_use::core::ConnectionReadyState;
use leptos_use::use_interval_fn;
use leptos_use::utils::Pausable;
use regex::Regex;

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

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

    create_effect(move |_| {
        let location = use_location();
        let mut navi = expect_context::<NavigationControllerSignal>();
        let pathname = (location.pathname)();
        let nanoid = if let Some(caps) = NANOID.captures(&pathname) {
            caps.name("nanoid").map(|m| m.as_str().to_string())
        } else {
            None
        };
        navi.update_nanoid(nanoid);
    });

    let api = ApiRequests::new();
    let ping = expect_context::<PingSignal>();
    let ws = expect_context::<WebsocketContext>();

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
                > 5
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
        <Meta name="color-scheme" content=color_scheme_meta/>
        <Html class=move || {
            match (color_scheme.prefers_dark)() {
                true => "dark",
                false => "",
            }
        }/>

        <Body/>
        <main class="min-h-screen w-full bg-light dark:bg-dark text-xs sm:text-sm md:text-md touch-manipulation select-none">
            <Header/>
            <Alert/>
            {children()}
        </main>
    }
}
