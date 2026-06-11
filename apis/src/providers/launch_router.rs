// Routes a cold-launched mobile app session.
//
// Three cold-launch shapes converge on the homepage:
//
//   1. App icon tap — no deep link, no notification. We route to the
//      user's "most urgent" game (least time on their clock where it's
//      their turn).
//
//   2. Notification tap — `tauri-plugin-notifications` fires its
//      `notificationClicked` event during plugin init, *before* our WASM
//      listener is up, so the FCM `data.link` is lost. We work around it
//      by pulling intent extras directly from the native activity via
//      the `launch_notification_extras` Tauri command (see
//      `culex/src-tauri/src/lib.rs` + `MainActivity.kt`), extract `link`,
//      and navigate.
//
//   3. Deep-link tap (https://hivegame.com/... or hive://...) — handled
//      by `DeepLinkListener` which navigates away from `/` before our
//      `Effect::watch` ever fires. We re-check `pathname == "/"` inside
//      the effect to stay safe if that order ever flips.
//
// Restricted to Tauri builds. On the SSR website / plain browser, a visit
// to `/` is an intentional navigation and the user expects the homepage.
//
// Ordering within this component: extras check first (specific —
// notification target), most-urgent fallback second (general — best
// guess for icon launches). The deep-link check happens implicitly via
// the `pathname == "/"` re-check.

#[cfg(not(feature = "ssr"))]
use {
    crate::{
        functions::games::get::most_urgent_game,
        providers::{auth_context::AuthContext, deep_link::deep_link_path},
    },
    leptos::prelude::*,
    leptos_router::{
        hooks::{use_location, use_navigate},
        NavigateOptions,
    },
    wasm_bindgen::{JsCast, JsValue},
    wasm_bindgen_futures::{spawn_local, JsFuture},
    web_sys::{
        console,
        js_sys::{Function, Promise, Reflect, JSON},
        window,
    },
};

#[cfg(feature = "ssr")]
use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
const CMD_LAUNCH_EXTRAS: &str = "launch_notification_extras";

#[cfg(not(feature = "ssr"))]
macro_rules! clog {
    ($($arg:tt)*) => {
        console::log_1(&JsValue::from_str(&format!($($arg)*)))
    };
}

#[component]
pub fn LaunchRouter() -> impl IntoView {
    #[cfg(not(feature = "ssr"))]
    {
        if !is_tauri() {
            return view! {};
        }

        let auth = expect_context::<AuthContext>();
        let navigate = use_navigate();
        let location = use_location();
        let already_routed = StoredValue::new(false);

        Effect::watch(
            move || auth.user.get(),
            move |current, _, _| {
                if already_routed.get_value() {
                    return;
                }
                if current.is_none() {
                    return;
                }
                let current_path = location.pathname.get_untracked();
                if current_path != "/" {
                    return;
                }
                already_routed.set_value(true);
                let nav = navigate.clone();
                spawn_local(async move {
                    if let Some(link) = take_notification_link().await {
                        if let Some(path) = deep_link_path(&link) {
                            clog!("launch: routing to notification target {path}");
                            nav(&path, NavigateOptions::default());
                            return;
                        }
                        clog!("launch: extras link {link} not parseable, falling back");
                    }
                    match most_urgent_game().await {
                        Ok(Some(game_id)) => {
                            clog!("launch: routing to most-urgent game /{}", game_id.0);
                            nav(&format!("/game/{}", game_id.0), NavigateOptions::default());
                        }
                        Ok(None) => {
                            clog!("launch: no urgent game, staying on home");
                        }
                        Err(err) => {
                            clog!("launch: most_urgent_game server fn failed: {err}");
                        }
                    }
                });
            },
            true,
        );
    }
    view! {}
}

/// Drain the native activity's launch-intent extras and pull the `link`
/// field. Returns None if there are no pending extras, the JSON doesn't
/// parse, or the blob has no `link` key — falling back to most-urgent
/// routing in each case.
#[cfg(not(feature = "ssr"))]
async fn take_notification_link() -> Option<String> {
    let json = invoke_string(CMD_LAUNCH_EXTRAS).await?;
    clog!("launch: launch_notification_extras returned {json}");
    let parsed = JSON::parse(&json).ok()?;
    let link = Reflect::get(&parsed, &JsValue::from_str("link")).ok()?;
    link.as_string()
}

#[cfg(not(feature = "ssr"))]
async fn invoke_string(command: &str) -> Option<String> {
    let win = window()?;
    let win_js: JsValue = win.into();
    let invoke = lookup_path(&win_js, &["__TAURI__", "core", "invoke"])?;
    let invoke_fn = invoke.dyn_into::<Function>().ok()?;
    let promise = invoke_fn
        .call1(&JsValue::null(), &JsValue::from_str(command))
        .ok()?
        .dyn_into::<Promise>()
        .ok()?;
    let resolved = JsFuture::from(promise).await.ok()?;
    if resolved.is_null() || resolved.is_undefined() {
        return None;
    }
    resolved.as_string()
}

#[cfg(not(feature = "ssr"))]
fn lookup_path(root: &JsValue, path: &[&str]) -> Option<JsValue> {
    let mut current = root.clone();
    for key in path {
        current = Reflect::get(&current, &JsValue::from_str(key)).ok()?;
        if current.is_undefined() || current.is_null() {
            return None;
        }
    }
    Some(current)
}

/// True iff the page is running inside a Tauri WebView (the global
/// `window.__TAURI__` object is injected by the Tauri runtime — same probe
/// we use in deep_link.rs / push_registration.rs).
#[cfg(not(feature = "ssr"))]
fn is_tauri() -> bool {
    let Some(win) = window() else {
        return false;
    };
    let win_js: JsValue = win.into();
    match Reflect::get(&win_js, &JsValue::from_str("__TAURI__")) {
        Ok(v) => !v.is_undefined() && !v.is_null(),
        Err(_) => false,
    }
}
