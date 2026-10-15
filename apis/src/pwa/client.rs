use super::PushError;
use crate::functions::{
    devices::{
        get_vapid_public_key,
        register_device,
        unregister_current_device,
        DeviceRegistration,
    },
    notification_preferences::send_test_push,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;
use std::cell::RefCell;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::{self, Function, Reflect, Uint8Array},
    Notification,
    NotificationPermission,
    PushSubscription,
    PushSubscriptionOptionsInit,
    ServiceWorkerRegistration,
};

pub const NUDGE_DISMISSED_KEY: &str = "hive-web-push-nudge-dismissed";
pub const INSTALL_NUDGE_DISMISSED_KEY: &str = "hive-install-nudge-dismissed";

#[derive(Deserialize)]
struct SubscriptionJson {
    endpoint: String,
    keys: SubscriptionKeys,
}

#[derive(Deserialize)]
struct SubscriptionKeys {
    p256dh: String,
    auth: String,
}

pub fn supported() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    Reflect::has(&window, &JsValue::from_str("Notification")).unwrap_or(false)
        && Reflect::has(&window.navigator(), &JsValue::from_str("serviceWorker")).unwrap_or(false)
        && Reflect::has(&window, &JsValue::from_str("PushManager")).unwrap_or(false)
}

thread_local! {
    static VAPID_KEY: RefCell<Option<Option<String>>> = const { RefCell::new(None) };
}

async fn cached_vapid_key() -> Option<String> {
    if let Some(cached) = VAPID_KEY.with(|c| c.borrow().clone()) {
        return cached;
    }
    match get_vapid_public_key().await {
        Ok(key) => {
            VAPID_KEY.with(|c| *c.borrow_mut() = Some(key.clone()));
            key
        }
        Err(_) => None,
    }
}

pub async fn push_available() -> bool {
    cached_vapid_key().await.is_some()
}

pub fn permission_blocked() -> bool {
    Notification::permission() == NotificationPermission::Denied
}

pub fn ios_needs_install() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    let ua = navigator.user_agent().unwrap_or_default().to_lowercase();
    let touch_points = Reflect::get(&navigator, &JsValue::from_str("maxTouchPoints"))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let is_ios = ua.contains("iphone")
        || ua.contains("ipad")
        || ua.contains("ipod")
        || (ua.contains("macintosh") && touch_points > 1.0);
    if !is_ios {
        return false;
    }
    let standalone = Reflect::get(&navigator, &JsValue::from_str("standalone"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    !standalone
}

pub async fn send_test() -> Result<(), PushError> {
    if Notification::permission() != NotificationPermission::Granted {
        return Err(PushError::EnableFirst);
    }
    send_test_push().await.map_err(|_| PushError::TestFailed)
}

pub async fn current_endpoint() -> Option<String> {
    current_subscription().await.map(|s| s.endpoint())
}

const NUDGE_SUPPRESS_MS: f64 = 30.0 * 24.0 * 60.0 * 60.0 * 1000.0;

fn dismissed_within_window(key: &str) -> bool {
    let stored = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item(key).ok().flatten());
    match stored.and_then(|s| s.parse::<f64>().ok()) {
        Some(ts) => web_sys::js_sys::Date::now() - ts < NUDGE_SUPPRESS_MS,
        None => false,
    }
}

fn mark_dismissed(key: &str) {
    if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = storage.set_item(key, &web_sys::js_sys::Date::now().to_string());
    }
}

pub fn nudge_dismissed() -> bool {
    dismissed_within_window(NUDGE_DISMISSED_KEY)
}

pub fn dismiss_nudge() {
    mark_dismissed(NUDGE_DISMISSED_KEY);
}

pub fn install_nudge_dismissed() -> bool {
    dismissed_within_window(INSTALL_NUDGE_DISMISSED_KEY)
}

pub fn dismiss_install_nudge() {
    mark_dismissed(INSTALL_NUDGE_DISMISSED_KEY);
}

pub fn is_standalone() -> bool {
    web_sys::window()
        .and_then(|w| Reflect::get(&w, &JsValue::from_str("__hiveStandalone")).ok())
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn has_install_prompt() -> bool {
    web_sys::window()
        .and_then(|w| Reflect::get(&w, &JsValue::from_str("__hiveInstallPrompt")).ok())
        .map(|v| !v.is_null() && !v.is_undefined())
        .unwrap_or(false)
}

pub fn install_nudge_should_show() -> bool {
    !is_standalone() && !install_nudge_dismissed() && (has_install_prompt() || ios_needs_install())
}

pub fn prompt_install() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(evt) = Reflect::get(&window, &JsValue::from_str("__hiveInstallPrompt")) else {
        return;
    };
    if evt.is_null() || evt.is_undefined() {
        return;
    }
    if let Ok(prompt_fn) = Reflect::get(&evt, &JsValue::from_str("prompt")) {
        if let Ok(func) = prompt_fn.dyn_into::<Function>() {
            let _ = func.call0(&evt);
        }
    }
    let _ = Reflect::set(
        &window,
        &JsValue::from_str("__hiveInstallPrompt"),
        &JsValue::NULL,
    );
}

/// Listen for `hive-navigate` messages posted by the service worker (e.g. when a
/// push notification is clicked) and hand the target path to `navigate`.
pub fn listen_for_navigation(navigate: impl Fn(String) + 'static) {
    use leptos_use::use_event_listener;
    use web_sys::MessageEvent;

    if !supported() {
        return;
    }
    let Some(sw) = web_sys::window().map(|w| w.navigator().service_worker()) else {
        return;
    };
    let _ = use_event_listener(sw, leptos::ev::message, move |ev: MessageEvent| {
        let data = ev.data();
        let field = |k: &str| {
            Reflect::get(&data, &JsValue::from_str(k))
                .ok()
                .and_then(|v| v.as_string())
        };
        if field("type").as_deref() == Some("hive-navigate") {
            if let Some(path) = field("path") {
                navigate(path);
            }
        }
    });
}

async fn registration() -> Result<ServiceWorkerRegistration, PushError> {
    let window = web_sys::window().ok_or(PushError::Unavailable)?;
    let navigator = window.navigator();
    if !Reflect::has(&navigator, &JsValue::from_str("serviceWorker")).unwrap_or(false) {
        return Err(PushError::ServiceWorkersUnsupported);
    }
    let lookup = navigator
        .service_worker()
        .get_registration_with_document_url("/");
    let registration = JsFuture::from(lookup)
        .await
        .map_err(|_| PushError::RegistrationLookupFailed)?;
    registration
        .dyn_into::<ServiceWorkerRegistration>()
        .map_err(|_| PushError::ServiceWorkerMissing)
}

async fn current_subscription() -> Option<PushSubscription> {
    let reg = registration().await.ok()?;
    let promise = reg
        .push_manager()
        .and_then(|pm| pm.get_subscription())
        .ok()?;
    let sub = JsFuture::from(promise).await.ok()?;
    sub.dyn_into::<PushSubscription>().ok()
}

pub async fn is_subscribed() -> bool {
    current_subscription().await.is_some()
}

pub async fn subscribe() -> Result<(), PushError> {
    if Notification::permission() == NotificationPermission::Denied {
        return Err(PushError::PermissionBlocked);
    }
    let permission = JsFuture::from(
        Notification::request_permission().map_err(|_| PushError::PermissionRequestFailed)?,
    )
    .await
    .map_err(|_| PushError::PermissionRequestFailed)?;
    if permission.as_string().as_deref() != Some("granted") {
        return Err(PushError::PermissionNotGranted);
    }

    let vapid_key = cached_vapid_key()
        .await
        .ok_or(PushError::ServerKeyMissing)?;

    let reg = registration().await?;
    let push_manager = reg
        .push_manager()
        .map_err(|_| PushError::PushManagerFailed)?;

    let key_bytes = URL_SAFE_NO_PAD
        .decode(vapid_key.trim())
        .map_err(|_| PushError::ServerKeyInvalid)?;
    let key_array = Uint8Array::from(key_bytes.as_slice());
    let options = PushSubscriptionOptionsInit::new();
    options.set_user_visible_only(true);
    options.set_application_server_key(key_array.as_ref());

    let subscription: PushSubscription = JsFuture::from(
        push_manager
            .subscribe_with_options(&options)
            .map_err(|_| PushError::SubscribeFailed)?,
    )
    .await
    .map_err(|_| PushError::SubscribeFailed)?
    .dyn_into()
    .map_err(|_| PushError::SubscriptionInvalid)?;

    register_subscription(&subscription, true).await
}

async fn register_subscription(
    subscription: &PushSubscription,
    explicit: bool,
) -> Result<(), PushError> {
    let json = js_sys::JSON::stringify(subscription).map_err(|_| PushError::SubscriptionInvalid)?;
    let parsed: SubscriptionJson =
        serde_json::from_str(&String::from(json)).map_err(|_| PushError::SubscriptionInvalid)?;

    let locale = web_sys::window()
        .and_then(|w| w.navigator().language())
        .unwrap_or_else(|| "en".to_string());
    register_device(DeviceRegistration {
        platform: "web".to_string(),
        device_token: parsed.endpoint,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        locale,
        p256dh: Some(parsed.keys.p256dh),
        auth: Some(parsed.keys.auth),
        explicit,
    })
    .await
    .map(|_| ())
    .map_err(|_| PushError::RegistrationFailed)
}

pub async fn reconcile_subscription() {
    if let Some(sub) = current_subscription().await {
        let _ = register_subscription(&sub, false).await;
    }
}

pub async fn unsubscribe() -> Result<(), PushError> {
    let Some(sub) = current_subscription().await else {
        return Ok(());
    };
    let endpoint = sub.endpoint();
    let released = JsFuture::from(
        sub.unsubscribe()
            .map_err(|_| PushError::UnsubscribeFailed)?,
    )
    .await
    .map_err(|_| PushError::UnsubscribeFailed)?
    .as_bool()
    .unwrap_or(false);
    if !released {
        return Err(PushError::SubscriptionNotReleased);
    }
    let _ = unregister_current_device(endpoint).await;
    Ok(())
}

pub async fn clear_local_subscription() {
    if let Some(sub) = current_subscription().await {
        if let Ok(p) = sub.unsubscribe() {
            let _ = JsFuture::from(p).await;
        }
    }
}
