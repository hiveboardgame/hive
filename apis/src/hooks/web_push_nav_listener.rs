use crate::pwa;
use leptos_router::hooks::use_navigate;

pub fn use_web_push_nav_listener() {
    let navigate = use_navigate();
    pwa::listen_for_navigation(move |path| navigate(&path, Default::default()));
}
