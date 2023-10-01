use crate::components::organisms::darkmode_toggle::DarkModeToggle;
use leptos::*;

#[component]
pub fn Header(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    view! {
        <header class=format!("w-full sticky top-0 flex justify-between {extend_tw_classes}")>
            <a href="/">
                Home
            </a>
            <a href="/play">
                Play
            </a>
            <a href="/hws">
                WebSocket
            </a>
            <a href="/sign_in">
            Sign in
            </a>
            <a href="/sign_up">
            Sign up
            </a>
            <a href="/logout">
            Logout
            </a>
            <a href="/user_account">
            User Account
            </a>
            <DarkModeToggle/>
        </header>
    }
}
