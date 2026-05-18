// Bridge between Tauri's deep-link plugin and the Leptos router.
//
// Two delivery paths converge here:
//
//   * Warm-receive (app already running): the Tauri shell's `on_open_url`
//     callback fires on each new URL and re-emits a "deep-link-opened"
//     Tauri event. The listener installed below picks it up.
//
//   * Cold-launch (app started by a deep link): `on_open_url` fires during
//     plugin init, BEFORE the WebView is loaded — that emit is lost. To
//     cover it, we also call the `current_deep_link` Tauri command on
//     mount and route any URL the plugin still has pending.
//
// We go through Tauri's event bus + a custom command instead of importing
// @tauri-apps/plugin-deep-link directly so the WASM frontend stays
// JS-bundle-free. Requires `withGlobalTauri: true` in tauri.conf.json so
// window.__TAURI__.event.listen / .core.invoke are exposed.
//
// In SSR builds and in plain-browser CSR (no Tauri wrapper), the component
// is a no-op: the window.__TAURI__ lookups return None and we silently
// skip both code paths.

#[cfg(not(feature = "ssr"))]
use {
    leptos::prelude::*,
    leptos_router::hooks::use_navigate,
    leptos_router::NavigateOptions,
    wasm_bindgen::{closure::Closure, JsCast, JsValue},
    wasm_bindgen_futures::{spawn_local, JsFuture},
    web_sys::js_sys::{Function, Promise, Reflect},
    web_sys::window,
};

#[cfg(feature = "ssr")]
use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
const TAURI_DEEP_LINK_EVENT: &str = "deep-link-opened";
#[cfg(not(feature = "ssr"))]
const TAURI_PULL_COMMAND: &str = "current_deep_link";
#[cfg(not(feature = "ssr"))]
const APP_HTTPS_PREFIX: &str = "https://hivegame.com";
#[cfg(not(feature = "ssr"))]
const APP_CUSTOM_SCHEME_PREFIX: &str = "hive://";

#[component]
pub fn DeepLinkListener() -> impl IntoView {
    #[cfg(not(feature = "ssr"))]
    {
        // Capture use_navigate() synchronously in the component body so the
        // Router Owner is bound now. Calling use_navigate() from inside an
        // async callback would panic — by then the Owner is gone. (Same
        // gotcha as the Login navigation; see the CSR owner-loss note in
        // the plan.)
        let navigate = use_navigate();
        let nav_for_pull = navigate.clone();

        // Cold-launch path: pull whatever URL the plugin already has.
        spawn_local(async move {
            if let Some(url) = invoke_current_deep_link().await {
                if let Some(path) = deep_link_path(&url) {
                    nav_for_pull(&path, NavigateOptions::default());
                }
            }
        });

        // Warm-receive path: subscribe to subsequent URLs via the event bus.
        install_tauri_listener(move |url: String| {
            if let Some(path) = deep_link_path(&url) {
                navigate(&path, NavigateOptions::default());
            }
        });
    }
    view! {}
}

#[cfg(not(feature = "ssr"))]
async fn invoke_current_deep_link() -> Option<String> {
    let win = window()?;
    let win_js: JsValue = win.into();
    let invoke = lookup_path(&win_js, &["__TAURI__", "core", "invoke"])?;
    let invoke_fn = invoke.dyn_into::<Function>().ok()?;
    let promise = invoke_fn
        .call1(&JsValue::null(), &JsValue::from_str(TAURI_PULL_COMMAND))
        .ok()?
        .dyn_into::<Promise>()
        .ok()?;
    JsFuture::from(promise).await.ok()?.as_string()
}

#[cfg(not(feature = "ssr"))]
fn install_tauri_listener<F>(callback: F)
where
    F: 'static + Fn(String),
{
    let Some(win) = window() else {
        return;
    };
    let win_js: JsValue = win.into();
    let Some(listen) = lookup_path(&win_js, &["__TAURI__", "event", "listen"]) else {
        // Not running inside Tauri (e.g. plain browser dev). Silent no-op —
        // deep links are an app-only feature.
        return;
    };
    let Ok(listen_fn) = listen.dyn_into::<Function>() else {
        return;
    };

    let cb = Closure::wrap(Box::new(move |event: JsValue| {
        if let Ok(payload) = Reflect::get(&event, &JsValue::from_str("payload")) {
            if let Some(s) = payload.as_string() {
                callback(s);
            }
        }
    }) as Box<dyn FnMut(JsValue)>);

    let _ = listen_fn.call2(
        &JsValue::null(),
        &JsValue::from_str(TAURI_DEEP_LINK_EVENT),
        cb.as_ref().unchecked_ref(),
    );
    // Leak: the listener lives for the full session. The component is mounted
    // exactly once inside the Router, so there's no unlisten/remount churn
    // to manage.
    cb.forget();
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

/// Convert an inbound deep-link URL into a router path.
/// Returns None for URLs we don't handle. `pub` so the push-notification
/// click listener in `push_registration.rs` can reuse the same parser for
/// `link` URLs delivered via FCM data payloads — same URL shapes, same
/// path extraction.
#[cfg(not(feature = "ssr"))]
pub fn deep_link_path(url: &str) -> Option<String> {
    if let Some(rest) = url.strip_prefix(APP_HTTPS_PREFIX) {
        return Some(if rest.is_empty() {
            "/".to_string()
        } else {
            rest.to_string()
        });
    }
    if let Some(rest) = url.strip_prefix(APP_CUSTOM_SCHEME_PREFIX) {
        // hive://game/abc  → /game/abc
        // hive:///game/abc → /game/abc (extra slash from authority-less URI)
        let trimmed = rest.trim_start_matches('/');
        return Some(format!("/{trimmed}"));
    }
    None
}

#[cfg(all(test, not(feature = "ssr")))]
mod tests {
    use super::deep_link_path;

    #[test]
    fn https_universal_link_extracts_path() {
        assert_eq!(
            deep_link_path("https://hivegame.com/game/abc"),
            Some("/game/abc".into())
        );
        assert_eq!(
            deep_link_path("https://hivegame.com/"),
            Some("/".into())
        );
        assert_eq!(
            deep_link_path("https://hivegame.com"),
            Some("/".into())
        );
    }

    #[test]
    fn custom_scheme_extracts_path() {
        assert_eq!(
            deep_link_path("hive://game/abc"),
            Some("/game/abc".into())
        );
        assert_eq!(
            deep_link_path("hive:///game/abc"),
            Some("/game/abc".into())
        );
    }

    #[test]
    fn unknown_origin_returns_none() {
        assert_eq!(deep_link_path("https://example.com/game/abc"), None);
        assert_eq!(deep_link_path("ftp://hivegame.com/x"), None);
    }
}
