use crate::components::atoms::og::OG;
use crate::components::atoms::title::Title;
use crate::components::molecules::alert::Alert;
use crate::components::organisms::header::Header;
use crate::providers::game_state::GameStateSignal;
use crate::providers::{ApiRequests, AuthContext, ColorScheme};

use crate::providers::navigation_controller::NavigationControllerSignal;
use crate::providers::refocus::RefocusSignal;
use crate::providers::websocket::WebsocketContext;
use crate::providers::PingSignal;
use cfg_if::cfg_if;
use chrono::Utc;
use hive_lib::GameControl;
use lazy_static::lazy_static;
use leptos::*;
use leptos_meta::*;
use leptos_router::use_location;
use leptos_use::core::ConnectionReadyState;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn, use_media_query, use_window_focus};
use regex::Regex;
cfg_if! { if #[cfg(not(feature = "ssr"))] {
    use leptos_use::utils::IS_IOS;
    use std::sync::RwLock;
    use web_sys::js_sys::Function;

    static IOS_WORKAROUND: RwLock<bool> = RwLock::new(false);
}}

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}
pub const COMMON_LINK_STYLE: &str = "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded";
pub const DROPDOWN_BUTTON_STYLE: &str= "h-full p-2 hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 whitespace-nowrap block";

#[derive(Clone)]
pub struct ControlsSignal {
    pub hidden: RwSignal<bool>,
    pub notify: Signal<bool>,
}

#[derive(Clone)]
pub struct OrientationSignal {
    pub is_tall: Signal<bool>,
    pub chat_dropdown_open: RwSignal<bool>,
    pub orientation_vertical: Signal<bool>,
}

#[component]
pub fn BaseLayout(children: ChildrenFn) -> impl IntoView {
    let color_scheme = expect_context::<ColorScheme>();
    let ping = expect_context::<PingSignal>();
    let ws = expect_context::<WebsocketContext>();
    let ws_ready = ws.ready_state;
    let auth_context = expect_context::<AuthContext>();
    let gamestate = expect_context::<GameStateSignal>();
    let stored_children = store_value(children);
    let is_tall = use_media_query("(min-height: 100vw)");
    let chat_dropdown_open = RwSignal::new(false);
    let orientation_vertical = Signal::derive(move || is_tall() || chat_dropdown_open());
    provide_context(OrientationSignal {
        is_tall,
        chat_dropdown_open,
        orientation_vertical,
    });

    //Copied from leptos-use https://github.com/Synphonyte/leptos-use/blob/main/src/on_click_outside.rs#L123-#L144
    #[cfg(not(feature = "ssr"))]
    {
        if *IS_IOS {
            if let Ok(mut ios_workaround) = IOS_WORKAROUND.write() {
                if !*ios_workaround {
                    *ios_workaround = true;
                    if let Some(body) = document().body() {
                        let children = body.children();
                        for i in 0..children.length() {
                            let _ = children
                                .get_with_index(i)
                                .expect("checked index")
                                .add_event_listener_with_callback("click", &Function::default());
                        }
                    }
                }
            }
        }
    }

    let color_scheme_meta = move || {
        if (color_scheme.prefers_dark)() {
            "dark".to_string()
        } else {
            "light".to_string()
        }
    };

    let user = move || match untrack(auth_context.user) {
        Some(Ok(Some(user))) => Some(user),
        _ => None,
    };

    let color = move || {
        if let Some(user) = user() {
            gamestate.user_color(user.id)
        } else {
            None
        }
    };
    let has_gamecontrol = create_read_slice(gamestate.signal, move |gs| {
        if let Some(color) = color() {
            let opp_color = color.opposite_color();
            matches!(
                gs.game_control_pending,
                Some(GameControl::TakebackRequest(color) | GameControl::DrawOffer(color)) if color == opp_color
            )
        } else {
            false
        }
    });
    let hide_controls = ControlsSignal {
        hidden: RwSignal::new(true),
        notify: has_gamecontrol,
    };

    provide_context(hide_controls);

    let is_hidden = RwSignal::new("hidden");
    create_effect(move |_| is_hidden.set(""));

    create_effect(move |_| {
        let location = use_location();
        let mut navi = expect_context::<NavigationControllerSignal>();
        let pathname = (location.pathname)();
        let nanoid = if let Some(caps) = NANOID.captures(&pathname) {
            caps.name("nanoid").map(|m| m.as_str().to_string())
        } else {
            None
        };
        if ws_ready() == ConnectionReadyState::Open {
            navi.update_nanoid(nanoid);
        };
    });

    let api = ApiRequests::new();

    let focused = use_window_focus();
    let _ = watch(
        focused,
        move |focused, _, _| {
            let mut refocus = expect_context::<RefocusSignal>();
            //log!("Focus changed");
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
            match ws_ready() {
                ConnectionReadyState::Closed => {
                    if retry_at.get() == counter.get() {
                        //log!("Reconnecting due to ReadyState");
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
                //log!("Reconnecting due to ping duration");
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
            content="width=device-width, initial-scale=1, interactive-widget=resizes-content, user-scalable=no"
        />
        <Html class=move || {
            match (color_scheme.prefers_dark)() {
                true => "dark",
                false => "",
            }
        }/>

        <Body/>
        <main class=move || {
            format!(
                "w-full min-h-screen text-xs bg-light dark:bg-gray-950 sm:text-sm touch-manipulations {}",
                is_hidden(),
            )
        }>
            <Header/>
            <Alert/>
            <Show when=move || ws_ready() != ConnectionReadyState::Open>
                <div class="absolute top-1/2 left-1/2 w-10 h-10 rounded-full border-t-2 border-b-2 border-blue-500 animate-spin"></div>
            </Show>
            {stored_children()()}

        </main>
    }
}
