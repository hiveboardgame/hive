use crate::organisms::darkmode::DarkModeToggle;
use leptos::*;

#[component]
pub fn Header(cx: Scope) -> impl IntoView {
    view! { cx,
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
            <DarkModeToggle/>
        </header>
    }
}
