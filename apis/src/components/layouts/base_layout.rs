use crate::components::atoms::{og::OG, title::Title};
use crate::components::molecules::alert::Alert;
use crate::components::molecules::tournament_ready_popup::TournamentReadyPopup;
use crate::components::organisms::header::Header;
use crate::providers::Config;
use crate::providers::{
    game_state::GameStateSignal, refocus::RefocusSignal, websocket::WebsocketContext, AuthContext,
    PingContext, UpdateNotifier,
};
use cfg_if::cfg_if;
use chrono::Utc;
use hive_lib::GameControl;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_use::core::ConnectionReadyState;
use leptos_use::utils::Pausable;
use leptos_use::{use_interval_fn, use_media_query, use_window_focus};

cfg_if! { if #[cfg(not(feature = "ssr"))] {
    use leptos_use::utils::IS_IOS;
    use std::sync::RwLock;
    use web_sys::js_sys::Function;

    static IOS_WORKAROUND: RwLock<bool> = RwLock::new(false);
}}

pub const COMMON_LINK_STYLE: &str = "no-link-style bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded";
pub const DROPDOWN_BUTTON_STYLE: &str= "font-bold h-full p-2 hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 whitespace-nowrap block";
pub const DROPDOWN_MENU_STYLE: &str = "flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md left-34 p-2 z-50";

#[derive(Clone)]
pub struct ControlsSignal {
    pub hidden: RwSignal<bool>,
    pub notify: Signal<bool>,
}

#[derive(Clone)]
pub struct OrientationSignal {
    pub chat_dropdown_open: RwSignal<bool>,
    pub orientation_vertical: Signal<bool>,
}

#[component]
pub fn BaseLayout(children: ChildrenFn) -> impl IntoView {
    provide_context(GameStateSignal::new());
    let config = expect_context::<Config>().0;
    let ping = expect_context::<PingContext>();
    let ws = expect_context::<WebsocketContext>();
    let ws_ready = ws.ready_state;
    let auth_context = expect_context::<AuthContext>();
    let gamestate = expect_context::<GameStateSignal>();
    let mut refocus = expect_context::<RefocusSignal>();
    let update_notifier = expect_context::<UpdateNotifier>();
    let vertical = use_media_query("(min-height: 100vw)");
    let chat_dropdown_open = RwSignal::new(false);
    let orientation_vertical = Signal::derive(move || vertical() || chat_dropdown_open());
    provide_context(OrientationSignal {
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

    let user_id = Signal::derive(move || {
        auth_context
            .user
            .with_untracked(|a| a.as_ref().map(|user| user.id))
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
            config
                .with(|cfg| match cfg.prefers_dark {
                    true => "dark",
                    false => "",
                })
        } />

        <Body />
        <main class=move || {
            format!(
                "w-full min-h-screen text-xs text-black dark:text-white bg-light dark:bg-gray-950 sm:text-sm touch-manipulation {}",
                is_hidden(),
            )
        }>
            <Header />
            <Alert />
            <TournamentReadyPopup ready_signal=update_notifier.tournament_ready />
            <Show when=move || ws_ready() != ConnectionReadyState::Open>
                <div class="flex absolute top-1/2 left-1/2 gap-2 items-center transform -translate-x-1/2 -translate-y-1/2">
                    <div class="w-10 h-10 rounded-full border-t-2 border-b-2 border-blue-500 animate-spin"></div>
                    <div class="text-lg font-bold text-ladybug-red">Connecting..</div>
                </div>
            </Show>
            {children()}

        </main>
    }
}
