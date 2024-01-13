use crate::components::organisms::header::Header;
use crate::providers::color_scheme::ColorScheme;
use crate::providers::navigation_controller::NavigationControllerSignal;
use crate::providers::web_socket::WebsocketContext;
use lazy_static::lazy_static;
use leptos::logging::log;
use leptos_use::use_interval_fn_with_options;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn, UseIntervalFnOptions};

use leptos::*;
use leptos_meta::*;
use leptos_router::use_location;
use leptos_use::core::ConnectionReadyState;
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

    let counter = create_rw_signal(0);

    let Pausable { pause, resume, .. } = use_interval_fn(
        move || {
            if counter.get() == 10 {
                let ws = expect_context::<WebsocketContext>();
                ws.open();
                log!("trying to reconnect");
            } else {
                log!("Counter is {}", counter.get_untracked());
                counter.update(|c| *c += 1);
            }
        },
        1000,
    );

    create_effect(move |_| {
        let websocket = expect_context::<WebsocketContext>();
        match websocket.ready_state.get() {
            ConnectionReadyState::Closed => {
                counter.update(|c| *c = 0);
                resume();
            }
            _ => {
                pause();
            }
        }
    });

    view! {
        <Meta name="color-scheme" content=color_scheme_meta/>
        <Html class=move || {
            match (color_scheme.prefers_dark)() {
                true => "dark",
                false => "",
            }
        }/>

        <Body/>
        <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>
        <main class="min-h-screen w-full bg-light dark:bg-dark text-xs sm:text-sm md:text-md lg:text-lg xl-text-xl">
            <Header/>
            {children()}
        </main>
    }
}
