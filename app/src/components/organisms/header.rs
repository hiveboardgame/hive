use crate::components::organisms::darkmode::DarkModeToggle;
use leptos::*;

#[component]
pub fn Header(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    view! {
        <header class=format!("sticky top-0 flex justify-between {extend_tw_classes}")>
            <a href="/">
                Home
            </a>
            <a href="/play">
                Play
            </a>
            <a href="/hws">
                WebSocket
            </a>
            <a href="/user_get">
                UserGet
            </a>
            <DarkModeToggle/>
        </header>
    }
}
