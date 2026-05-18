// Bridge between the OS push subsystem (APNs/FCM, exposed by
// `tauri-plugin-notifications`) and our server-side device registry.
//
// On user login, we:
//   1. Detect platform from the user agent ("fcm" / "apns").
//   2. Call the plugin's `registerForPushNotifications` Tauri command via the
//      global Tauri JS bridge. The plugin asks for permission, registers
//      with the OS, and returns the device token.
//   3. POST that to our `register_device` server fn (bearer-authenticated
//      via the same JWT path other mobile calls use), which upserts a row
//      in `push_devices`.
//   4. Persist the returned device id in localStorage so the logout flow
//      can forward it to the Logout server fn for row cleanup.
//
// On logout the server-side cleanup happens inside the Logout server fn
// itself: the Logout button component reads the stored device id and
// passes it as a hidden form field, and the server fn deletes the row
// while the bearer is still attached. Here, we only clear the local
// pointer so a subsequent re-login starts from a clean state. We do *not*
// call the server again — by the time our `auth.user` watcher fires, the
// bearer has already been cleared client-side and a separate server call
// would fail authentication.
//
// In SSR builds and plain-browser CSR (no Tauri wrapper), the component
// is a no-op: platform detection returns None on desktop and the Tauri
// invoke path silently returns None.
//
// iOS is paused at the project level (see memory entry `ios_push_paused`);
// the iOS code path is left in place so resuming iOS push later doesn't
// require touching this file.

#[cfg(not(feature = "ssr"))]
use {
    crate::{
        functions::devices::register_device,
        providers::{auth_context::AuthContext, deep_link::deep_link_path},
    },
    leptos::prelude::*,
    leptos_router::{hooks::use_navigate, NavigateOptions},
    wasm_bindgen::{closure::Closure, JsCast, JsValue},
    wasm_bindgen_futures::{spawn_local, JsFuture},
    web_sys::{
        console,
        js_sys::{Function, Object, Promise, Reflect},
        window,
    },
};

// Project-wide `log` macros are not wired to a wasm-side logger, so they
// drop. Until that lands, push debug messages directly to the WebView
// console — visible in chrome://inspect attached to the Android WebView,
// or via `adb logcat | grep chromium`.
#[cfg(not(feature = "ssr"))]
macro_rules! clog {
    ($($arg:tt)*) => {
        console::log_1(&JsValue::from_str(&format!($($arg)*)))
    };
}
#[cfg(not(feature = "ssr"))]
macro_rules! cwarn {
    ($($arg:tt)*) => {
        console::warn_1(&JsValue::from_str(&format!($($arg)*)))
    };
}

#[cfg(feature = "ssr")]
use leptos::prelude::*;

#[cfg(not(feature = "ssr"))]
const STORAGE_KEY: &str = "hive-push-device-id";
// Tauri command identifiers use the literal Rust function name (snake_case),
// not the camelCase JS-wrapper name. Plugin name is "notifications" (matches
// the `notifications:default` capability we declared).
#[cfg(not(feature = "ssr"))]
const CMD_IS_PERMISSION_GRANTED: &str = "plugin:notifications|is_permission_granted";
#[cfg(not(feature = "ssr"))]
const CMD_REQUEST_PERMISSION: &str = "plugin:notifications|request_permission";
#[cfg(not(feature = "ssr"))]
const CMD_REGISTER_PUSH: &str = "plugin:notifications|register_for_push_notifications";
#[cfg(not(feature = "ssr"))]
const CMD_SET_CLICK_LISTENER_ACTIVE: &str = "plugin:notifications|set_click_listener_active";
// Tauri event the notifications plugin emits when the user taps a
// notification body (not action buttons — those go through the separate
// `actionPerformed` event). Format matches the plugin convention
// `plugin:<plugin_name>:<event>`. The payload is `{ id: number, data?:
// Record<string, string> }`; `data` is exactly our FCM `data` block, so
// `data.link` is the canonical deep-link URL the server attached.
#[cfg(not(feature = "ssr"))]
const TAURI_NOTIFICATION_CLICKED_EVENT: &str = "plugin:notifications:notificationClicked";

#[component]
pub fn PushRegistrationListener() -> impl IntoView {
    #[cfg(not(feature = "ssr"))]
    {
        // Must be inside <Router> AND inside the AuthContext provider — the
        // call site in app.rs mounts us next to DeepLinkListener after
        // provide_auth(), so both are in scope.
        let auth = expect_context::<AuthContext>();

        // Capture `use_navigate()` synchronously in the component body so
        // the Router Owner is bound now — calling it from inside an async
        // closure later would panic. Same gotcha that DeepLinkListener
        // documents for its event listener.
        install_notification_click_listener(use_navigate());

        Effect::watch(
            move || auth.user.get(),
            move |current, previous, _| {
                // `previous` is None on the first immediate fire (app launch).
                // We treat that as "was not logged in" so a session restored
                // from localStorage at startup triggers a (re-)registration.
                let was_logged_in = matches!(previous, Some(Some(_)));
                let is_logged_in = current.is_some();
                clog!(
                    "push: auth transition (was_logged_in={was_logged_in}, is_logged_in={is_logged_in})"
                );
                match (was_logged_in, is_logged_in) {
                    (false, true) => spawn_local(register_with_server()),
                    (true, false) => {
                        // Server-side push_devices cleanup is handled by the
                        // Logout server fn — it receives the device_id via a
                        // hidden form field and deletes the row while the
                        // bearer is still valid. Here we only discard the
                        // local pointer so a subsequent re-login starts from
                        // a clean state and doesn't reference a deleted row.
                        if let Some(storage) =
                            window().and_then(|w| w.local_storage().ok().flatten())
                        {
                            let _ = storage.remove_item(STORAGE_KEY);
                            clog!("push: logout — cleared local device id");
                        }
                    }
                    _ => {}
                }
            },
            true,
        );
    }
    view! {}
}

#[cfg(not(feature = "ssr"))]
async fn register_with_server() {
    let Some(platform) = detect_platform() else {
        clog!("push: not a mobile platform, skipping registration");
        return;
    };
    clog!("push: detected platform = {platform}");

    // Query the current permission state before prompting. Calling
    // `request_permission` a second time within the same app lifecycle when
    // permission is already granted hangs the plugin's Android side: the OS
    // doesn't fire `onRequestPermissionsResult` for an already-granted
    // permission, so the JS promise never settles. `is_permission_granted`
    // is a pure state query and is safe to call repeatedly.
    //
    //   Some(true)  → granted, skip the prompt
    //   Some(false) → previously denied, abort (no point spamming the user)
    //   None        → state unknown (Prompt / PromptWithRationale) → ask
    clog!("push: calling is_permission_granted");
    let granted: Option<bool> = invoke_value(CMD_IS_PERMISSION_GRANTED)
        .await
        .and_then(|v| {
            if v.is_null() || v.is_undefined() {
                None
            } else {
                v.as_bool()
            }
        });
    clog!("push: is_permission_granted = {granted:?}");

    match granted {
        Some(true) => {}
        Some(false) => {
            cwarn!("push: permission previously denied, aborting registration");
            return;
        }
        None => {
            clog!("push: requesting permission");
            let perm = invoke_value(CMD_REQUEST_PERMISSION)
                .await
                .and_then(|v| v.as_string());
            clog!("push: request_permission returned {perm:?}");
            if !matches!(perm.as_deref(), Some("granted")) {
                cwarn!("push: permission not granted ({perm:?}), aborting");
                return;
            }
        }
    }

    clog!("push: calling register_for_push_notifications");
    let token = match invoke_value(CMD_REGISTER_PUSH)
        .await
        .and_then(|v| v.as_string())
    {
        Some(t) => {
            let prefix: String = t.chars().take(8).collect();
            clog!("push: got token (prefix {prefix}…, len {})", t.len());
            t
        }
        None => {
            cwarn!("push: register_for_push_notifications returned no token");
            return;
        }
    };

    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let locale = window()
        .and_then(|w| w.navigator().language())
        .unwrap_or_else(|| "en".to_string());

    clog!("push: calling register_device server fn");
    match register_device(platform.to_string(), token, app_version, locale).await {
        Ok(device_id) => {
            clog!("push: registered, device_id = {device_id}");
            if let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) {
                let _ = storage.set_item(STORAGE_KEY, &device_id.to_string());
            }
        }
        Err(err) => cwarn!("push: register_device server fn failed: {err}"),
    }
}

#[cfg(not(feature = "ssr"))]
fn detect_platform() -> Option<&'static str> {
    let ua = window()?.navigator().user_agent().ok()?;
    if ua.contains("Android") {
        Some("fcm")
    } else if ua.contains("iPhone") || ua.contains("iPad") {
        Some("apns")
    } else {
        None
    }
}

/// Invoke a Tauri command and return its resolved JS value. Returns None if
/// the JS bridge isn't present, the command isn't registered, or the promise
/// rejects. Callers apply the appropriate `as_bool` / `as_string` extractor.
#[cfg(not(feature = "ssr"))]
async fn invoke_value(command: &str) -> Option<JsValue> {
    let win = window()?;
    let win_js: JsValue = win.into();
    let invoke = lookup_path(&win_js, &["__TAURI__", "core", "invoke"])?;
    let invoke_fn = invoke.dyn_into::<Function>().ok()?;
    let promise = invoke_fn
        .call1(&JsValue::null(), &JsValue::from_str(command))
        .ok()?
        .dyn_into::<Promise>()
        .ok()?;
    match JsFuture::from(promise).await {
        Ok(value) => Some(value),
        Err(err) => {
            cwarn!("push: invoke {command} rejected: {err:?}");
            None
        }
    }
}

/// Wire up the notification-tap → deep-link-navigate flow.
///
/// Tapping an FCM notification with a `notification` block on Android
/// doesn't fire any URL intent — the OS just launches the app's launcher
/// activity. Our existing `DeepLinkListener` (subscribed to the
/// `tauri-plugin-deep-link` events) never sees it. Instead the
/// notifications plugin emits a separate `notificationClicked` event
/// with the FCM `data` block attached. We subscribe to that, pull the
/// `link` URL, parse it via the shared `deep_link_path` parser, and
/// navigate.
///
/// `set_click_listener_active(true)` is the explicit opt-in the plugin
/// requires before it starts emitting these events. Fire-and-forget — a
/// failure means the listener stays installed but receives nothing; the
/// app still functions, just without tap routing.
#[cfg(not(feature = "ssr"))]
fn install_notification_click_listener(navigate: impl Fn(&str, NavigateOptions) + 'static) {
    let Some(win) = window() else {
        return;
    };
    let win_js: JsValue = win.into();
    let Some(listen) = lookup_path(&win_js, &["__TAURI__", "event", "listen"]) else {
        clog!("push: __TAURI__.event.listen not present — skipping click listener");
        return;
    };
    let Ok(listen_fn) = listen.dyn_into::<Function>() else {
        return;
    };

    let cb = Closure::wrap(Box::new(move |event: JsValue| {
        let Some(url) = extract_click_link(&event) else {
            clog!("push: click event missing data.link");
            return;
        };
        let Some(path) = deep_link_path(&url) else {
            cwarn!("push: click link not routable: {url}");
            return;
        };
        clog!("push: tap → navigating to {path}");
        navigate(&path, NavigateOptions::default());
    }) as Box<dyn FnMut(JsValue)>);

    let _ = listen_fn.call2(
        &JsValue::null(),
        &JsValue::from_str(TAURI_NOTIFICATION_CLICKED_EVENT),
        cb.as_ref().unchecked_ref(),
    );
    // Leak the closure: the listener lives for the full session. The
    // component is mounted exactly once inside the Router, so there's no
    // unlisten/remount churn to manage.
    cb.forget();

    // Tell the plugin to start emitting `notificationClicked` events.
    // Without this call, the listener above is silent.
    spawn_local(async {
        let _ = invoke_with_active(CMD_SET_CLICK_LISTENER_ACTIVE, true).await;
    });
}

/// Invoke a Tauri command that takes a single `{ active: bool }` argument.
/// Used for `set_click_listener_active`. Returns None on any failure of
/// the JS bridge — callers treat the activation as best-effort.
#[cfg(not(feature = "ssr"))]
async fn invoke_with_active(command: &str, active: bool) -> Option<JsValue> {
    let win = window()?;
    let win_js: JsValue = win.into();
    let invoke = lookup_path(&win_js, &["__TAURI__", "core", "invoke"])?;
    let invoke_fn = invoke.dyn_into::<Function>().ok()?;
    let args = Object::new();
    Reflect::set(
        &args,
        &JsValue::from_str("active"),
        &JsValue::from_bool(active),
    )
    .ok()?;
    let promise = invoke_fn
        .call2(
            &JsValue::null(),
            &JsValue::from_str(command),
            &JsValue::from(args),
        )
        .ok()?
        .dyn_into::<Promise>()
        .ok()?;
    match JsFuture::from(promise).await {
        Ok(v) => Some(v),
        Err(err) => {
            cwarn!("push: invoke {command} rejected: {err:?}");
            None
        }
    }
}

/// Extract `event.payload.data.link` as a String. The plugin payload shape
/// is `{ id: number, data?: Record<string, string> }`; we put the URL in
/// `data.link` on the server side (see `notifications::event::render_push`).
#[cfg(not(feature = "ssr"))]
fn extract_click_link(event: &JsValue) -> Option<String> {
    let payload = Reflect::get(event, &JsValue::from_str("payload")).ok()?;
    let data = Reflect::get(&payload, &JsValue::from_str("data")).ok()?;
    if data.is_null() || data.is_undefined() {
        return None;
    }
    let link = Reflect::get(&data, &JsValue::from_str("link")).ok()?;
    link.as_string()
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
