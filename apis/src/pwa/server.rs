//! SSR-safe no-op surface. The real browser implementation lives in `client.rs`
//! and only compiles for the wasm target; on the server these stand in so UI
//! components can call the `pwa` facade without `#[cfg]` of their own.

pub fn supported() -> bool {
    false
}

pub fn permission_blocked() -> bool {
    false
}

pub fn ios_needs_install() -> bool {
    false
}

pub fn is_standalone() -> bool {
    false
}

pub fn nudge_dismissed() -> bool {
    false
}

pub fn dismiss_nudge() {}

pub fn install_nudge_dismissed() -> bool {
    false
}

pub fn dismiss_install_nudge() {}

pub fn install_nudge_should_show() -> bool {
    false
}

pub fn prompt_install() {}

pub async fn push_available() -> bool {
    false
}

pub async fn is_subscribed() -> bool {
    false
}

pub async fn current_endpoint() -> Option<String> {
    None
}

pub async fn subscribe() -> Result<(), String> {
    Err("not available server-side".into())
}

pub async fn unsubscribe() -> Result<(), String> {
    Err("not available server-side".into())
}

pub async fn send_test() -> Result<(), String> {
    Err("not available server-side".into())
}

pub async fn reconcile_subscription() {}

pub async fn clear_local_subscription() {}

pub fn listen_for_navigation(_navigate: impl Fn(String) + 'static) {}
