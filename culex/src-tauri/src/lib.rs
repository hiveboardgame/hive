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
        .invoke_handler(tauri::generate_handler![current_deep_link])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
