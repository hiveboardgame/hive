use crate::{
    components::{
        atoms::{og::OG, title::Title},
        molecules::{
            alert::Alert,
            install_nudge::InstallNudge,
            tournament_ready_popup::TournamentReadyPopup,
            web_push_nudge::WebPushNudge,
        },
        organisms::header::Header,
    },
    hooks::{
        install_nudge::use_install_nudge_active,
        sync_user_locale::use_sync_user_locale,
        web_push_nav_listener::use_web_push_nav_listener,
        web_push_reconcile::use_web_push_reconcile,
    },
    providers::{
        game_state::GameStateSignal,
        refocus::RefocusSignal,
        websocket::{ConnectionReadyState, WebsocketContext},
        AuthContext,
        Config,
        PingContext,
        UpdateNotifier,
        FRESH_WINDOW_SECS,
    },
};
use cfg_if::cfg_if;
use chrono::Utc;
use hive_lib::GameControl;
use leptos::prelude::*;
use leptos_meta::*;
use leptos_use::{use_interval_fn, use_media_query, use_window_focus, utils::Pausable};

cfg_if! { if #[cfg(not(feature = "ssr"))] {
    use leptos_use::utils::IS_IOS;
    use std::sync::RwLock;
    use web_sys::js_sys::Function;

    static IOS_WORKAROUND: RwLock<bool> = RwLock::new(false);
}}

pub const COMMON_LINK_STYLE: &str = "no-link-style bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 m-1 rounded";
pub const DROPDOWN_BUTTON_STYLE: &str= "font-bold h-full p-2 hover:bg-pillbug-teal dark:hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 whitespace-nowrap block";
pub const DROPDOWN_MENU_STYLE: &str = "flex flex-col items-stretch absolute left-0 top-full mt-1 w-max bg-even-light dark:bg-gray-950 text-black border border-gray-300 rounded-md p-2 z-50";

#[derive(Clone)]
pub struct ControlsSignal {
    pub hidden: RwSignal<bool>,
    pub notify: Signal<bool>,
}

#[derive(Clone)]
pub struct OrientationSignal {
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
    let orientation_vertical = use_media_query("(orientation: portrait), (max-width: 640px)");
    provide_context(OrientationSignal {
        orientation_vertical,
    });
    use_sync_user_locale();
    use_web_push_nav_listener();
    use_web_push_reconcile();
    let install_nudge_active = use_install_nudge_active();

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
        true,
    );

    // Zombie-socket detector: socket says Open but we haven't seen a
    // server ping in a while. Force a fresh connection — but only with a
    // backoff between attempts and a grace window after each reopen, so a
    // server-wide ping hiccup doesn't cause every visible client to
    // reconnect every tick in lockstep.
    const REOPEN_MIN_GAP_MS: i64 = 500;
    const REOPEN_MAX_GAP_MS: i64 = 30_000;
    const REOPEN_JITTER_MAX_MS: i64 = 500;
    let last_reopen_at = StoredValue::new(None::<chrono::DateTime<Utc>>);
    let next_reopen_gap_ms = StoredValue::new(REOPEN_MIN_GAP_MS);
    let Pausable { .. } = use_interval_fn(
        move || {
            let ws = ws.clone();
            // Untracked: this closure runs on a 1Hz timer, not as a reactive
            // effect. Subscribing to ws_ready/last_updated would re-fire the
            // body on every state transition, double-triggering the detector.
            if ws_ready.get_untracked() != ConnectionReadyState::Open {
                // Connecting/Closing/Closed: let the WS provider's own
                // backoff handle it. Don't reset our own state — if we're
                // mid-reopen we want the cooldown to keep growing on
                // sustained failure.
                return;
            }
            let now = Utc::now();
            let last_ping = ping.last_updated.get_untracked();
            let stale = now.signed_duration_since(last_ping).num_seconds() >= FRESH_WINDOW_SECS;
            if !stale {
                // Re-arm the backoff only when freshness comes from a *real*
                // server ping. After a reopen we synthetically advance
                // last_updated to grant a grace window, and that artificial
                // freshness must not reset the backoff — otherwise sustained
                // failures keep retrying at MIN_GAP because every other tick
                // looks fresh.
                let real_ping_seen = match last_reopen_at.get_value() {
                    Some(t_reopen) => last_ping > t_reopen,
                    None => true,
                };
                if real_ping_seen {
                    next_reopen_gap_ms.set_value(REOPEN_MIN_GAP_MS);
                }
                return;
            }
            // Jitter spreads simultaneous reopens across clients during a
            // server-wide hiccup so we don't thunder-herd the reconnect.
            let jitter = (web_sys::js_sys::Math::random() * REOPEN_JITTER_MAX_MS as f64) as i64;
            let required_gap = next_reopen_gap_ms.get_value() + jitter;
            if let Some(prev) = last_reopen_at.get_value() {
                if now.signed_duration_since(prev).num_milliseconds() < required_gap {
                    return;
                }
            }
            ws.open();
            last_reopen_at.set_value(Some(now));
            // Grace window: the just-reopened socket needs FRESH_WINDOW_SECS
            // to receive its first server ping; without this bump, the next
            // staleness check fires immediately on stale `last_updated`.
            ping.mark_active();
            let new_gap = (next_reopen_gap_ms.get_value() * 2).min(REOPEN_MAX_GAP_MS);
            next_reopen_gap_ms.set_value(new_gap);
        },
        1000,
    );
    view! {
        <Title />
        <OG />
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
                "w-full min-h-screen standalone:min-h-[var(--app-height)] text-xs text-black dark:text-white bg-light dark:bg-gray-950 sm:text-sm touch-manipulation {}",
                is_hidden(),
            )
        }>
            <Header />
            <Alert />
            <InstallNudge active=install_nudge_active />
            <WebPushNudge install_nudge_active=install_nudge_active />
            <TournamentReadyPopup ready_signal=update_notifier.tournament_ready />
            <Show when=move || ws_ready() != ConnectionReadyState::Open>
                <div class="flex absolute top-1/2 left-1/2 gap-2 items-center -translate-x-1/2 -translate-y-1/2 z-[60]">
                    <div class="rounded-full border-t-2 border-b-2 border-blue-500 animate-spin size-10"></div>
                    <div class="text-lg font-bold text-ladybug-red">Connecting..</div>
                </div>
            </Show>
            {children()}

        </main>
    }
}
