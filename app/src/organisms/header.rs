use crate::organisms::darkmode::DarkModeToggle;
use leptos::*;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="sticky top-0 flex justify-between">
            <a href="/">
                Home
            </a>
            <a href="/play">
                Play
            </a>
            <a href="/hws">
                WebSocket
            </a>
            <a href="/user">
                User
            </a>
            <a href="/user_get">
                UserGet
            </a>
            <DarkModeToggle/>
        </header>
    }
}
