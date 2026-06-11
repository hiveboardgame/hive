use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

// Tauri event we re-emit each inbound deep link on. The Leptos frontend
// subscribes via window.__TAURI__.event.listen and routes via use_navigate
// (see apis/src/providers/deep_link.rs). Keeping the URL plumbing on the
// Rust side means the frontend doesn't need a JS dependency on the
// plugin — just the standard Tauri event bus.
//
// Cold-start race: on a deep-link-triggered cold launch, on_open_url
// fires during plugin init, before the WebView is loaded. The emit is
// then lost. To cover that case, the frontend ALSO calls the
// `current_deep_link` Tauri command on mount and routes any URL the
// plugin still has pending. Warm-receive (app already running) hits the
// event path; cold-launch hits the pull path; both routes converge.
const DEEP_LINK_EVENT: &str = "deep-link-opened";

#[tauri::command]
fn current_deep_link(app: tauri::AppHandle) -> Option<String> {
    app.deep_link()
        .get_current()
        .ok()
        .flatten()
        .and_then(|urls| urls.into_iter().next())
        .map(|u| u.to_string())
}

// Holds the most recent FCM data-block JSON the activity has captured.
// `MainActivity.kt` pushes here via `LaunchExtrasBridge.put` (whose JNI
// export lives below in the Android module). The Tauri command drains
// the cell so a re-mount of the WASM launch router doesn't re-route to
// the same notification target.
static LAUNCH_EXTRAS: Mutex<Option<String>> = Mutex::new(None);

// Same cold-start problem as `current_deep_link`, but for the notifications
// plugin: when a notification tap launches the app from killed, the
// plugin's `notificationClicked` event fires during init before the
// WebView listens, and the FCM `data.link` is lost.
//
// Direction of the workaround: Kotlin captures the launch-intent extras
// on `onCreate` / `onNewIntent` and pushes them into Rust via the JNI
// export below — *not* the other way around, because pulling from Rust
// requires `ndk-context`'s global which Tauri/wry don't initialise.
//
// This command returns the raw JSON snapshot (or None if there are no
// pending extras) and clears the cell. Parsing lives on the WASM side so
// the Rust shell stays payload-agnostic.
#[tauri::command]
fn launch_notification_extras() -> Option<String> {
    LAUNCH_EXTRAS.lock().ok()?.take()
}

// JNI export called by `LaunchExtrasBridge.put` from MainActivity. Stores
// the JSON string into `LAUNCH_EXTRAS`. `extern "system"` (which Rust
// treats as `extern "C"` everywhere it matters) lets the JVM dispatch
// here directly without a Rust panic crossing the FFI boundary —
// `get_string` failures are swallowed rather than panicked.
//
// Signature: `@JvmStatic` on a Kotlin companion produces a static JVM
// method, so the second arg is `JClass`, not `JObject`.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_hivegame_culex_LaunchExtrasBridge_put<'local>(
    mut env: jni::JNIEnv<'local>,
    _class: jni::objects::JClass<'local>,
    json: jni::objects::JString<'local>,
) {
    let Ok(java_str) = env.get_string(&json) else {
        log::warn!("LaunchExtrasBridge.put: get_string failed, dropping extras");
        return;
    };
    let s: String = java_str.into();
    if let Ok(mut guard) = LAUNCH_EXTRAS.lock() {
        *guard = Some(s);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_notifications::init())
        .setup(|app| {
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    if let Err(err) = handle.emit(DEEP_LINK_EVENT, url.to_string()) {
                        log::warn!("failed to emit {DEEP_LINK_EVENT}: {err}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            current_deep_link,
            launch_notification_extras
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
