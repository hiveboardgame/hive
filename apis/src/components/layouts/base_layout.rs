use crate::components::atoms::{og::OG, title::Title};
use crate::components::molecules::alert::Alert;
use crate::components::organisms::header::Header;
use crate::providers::Config;
use crate::providers::{
    game_state::GameStateSignal, navigation_controller::NavigationControllerSignal,
    refocus::RefocusSignal, websocket::WebsocketContext, AuthContext, PingContext,
};
use cfg_if::cfg_if;
use chrono::Utc;
use hive_lib::GameControl;
use lazy_static::lazy_static;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::hooks::use_location;
use leptos_use::core::ConnectionReadyState;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn, use_media_query, use_window_focus};
use regex::Regex;
use shared_types::{GameId, TournamentId};

cfg_if! { if #[cfg(not(feature = "ssr"))] {
    use leptos_use::utils::IS_IOS;
    use std::sync::RwLock;
    use web_sys::js_sys::Function;

    static IOS_WORKAROUND: RwLock<bool> = RwLock::new(false);
}}

lazy_static! {
    static ref GAME_NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

lazy_static! {
    static ref TOURNAMENT_NANOID: Regex =
        Regex::new(r"/tournament/(?<nanoid>.*)").expect("This regex should compile");
}
pub const COMMON_LINK_STYLE: &str = "bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded";
pub const DROPDOWN_BUTTON_STYLE: &str= "font-bold h-full p-2 hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 whitespace-nowrap block";

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
    let config = expect_context::<Config>().0;
    let ping = expect_context::<PingContext>();
    let ws = expect_context::<WebsocketContext>();
    let ws_ready = ws.ready_state;
    let auth_context = expect_context::<AuthContext>();
    let gamestate = expect_context::<GameStateSignal>();
    let mut refocus = expect_context::<RefocusSignal>();
    let stored_children = Signal::derive(move || children.clone());
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
        if config().prefers_dark {
            "dark".to_string()
        } else {
            "light".to_string()
        }
    };

    let user_id = Signal::derive(move || match auth_context.user.get_untracked() {
        Some(Ok(user)) => Some(user.id),
        _ => None,
    });

    let user_color = gamestate.user_color_as_signal(user_id);
    let has_gamecontrol = create_read_slice(gamestate.signal, move |gs| {
        if let Some(color) = user_color() {
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
    Effect::new(move |_| is_hidden.set(""));

    Effect::new(move |_| {
        let location = use_location();
        let mut navi = expect_context::<NavigationControllerSignal>();
        let pathname = (location.pathname)();

        let game_id = if let Some(caps) = GAME_NANOID.captures(&pathname) {
            caps.name("nanoid").map(|m| GameId(m.as_str().to_string()))
        } else {
            None
        };
        let tournament_id = if let Some(caps) = TOURNAMENT_NANOID.captures(&pathname) {
            caps.name("nanoid")
                .map(|m| TournamentId(m.as_str().to_string()))
        } else {
            None
        };

        if ws_ready() == ConnectionReadyState::Open {
            navi.update_ids(game_id, tournament_id);
        };
    });

    let focused = use_window_focus();
    let _ = Effect::watch(
        focused,
        move |focused, _, _| {
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
            let ws = ws.clone();
            counter.update(|c| *c += 1);
            match ws_ready() {
                ConnectionReadyState::Closed => {
                    if retry_at.get() == counter.get() {
                        //log!("Reconnecting due to ReadyState");
                        ws.open();
                        counter.update(|c| *c = 0);
                        retry_at.update(|r| *r *= 2);
                    }
                }
                ConnectionReadyState::Open => {
                    counter.update(|c| *c = 0);
                    retry_at.update(|r| *r = 2);
                }
                _ => {}
            }
            if Utc::now()
                .signed_duration_since(ping.last_updated.get_untracked())
                .num_seconds()
                >= 5
                && retry_at.get() == counter.get()
            {
                //log!("Reconnecting due to ping duration");
                ws.open();
                counter.update(|c| *c = 0);
                retry_at.update(|r| *r *= 2);
            };
        },
        1000,
    );
    view! {
        <Title />
        <OG />
        <Meta name="color-scheme" attr:content=color_scheme_meta />
        <Meta
            name="viewport"
            content="width=device-width, initial-scale=1, interactive-widget=resizes-content, user-scalable=no"
        />
        <Link rel="manifest" href="/assets/site.webmanifest" />
        <Link rel="apple-touch-icon" href="/assets/android-chrome-192x192.png" />
        <Meta name="mobile-web-app-capable" content="yes" />
        <Meta name="apple-mobile-web-app-status-bar-style" content="black" />
        <Script src="/assets/js/pwa.js" />
        <Html attr:class=move || {
            match config().prefers_dark {
                true => "dark",
                false => "",
            }
        } />

        <Body />
        <main class=move || {
            format!(
                "w-full min-h-screen text-xs bg-light dark:bg-gray-950 sm:text-sm touch-manipulation {}",
                is_hidden(),
            )
        }>
            <Header />
            <Alert />
            <Show when=move || ws_ready() != ConnectionReadyState::Open>
                <div class="flex absolute top-1/2 left-1/2 gap-2 items-center transform -translate-x-1/2 -translate-y-1/2">
                    <div class="w-10 h-10 rounded-full border-t-2 border-b-2 border-blue-500 animate-spin"></div>
                    <div class="text-lg font-bold text-ladybug-red">Connecting..</div>
                </div>
            </Show>
            {stored_children()()}

        </main>
    }
}
