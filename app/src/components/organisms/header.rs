use crate::components::organisms::darkmode_toggle::DarkModeToggle;
use crate::providers::auth_context::*;
use leptos::*;

#[component]
pub fn Header(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = use_context::<AuthContext>().expect("Failed to get AuthContext");
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
            <Transition fallback=move || ()>
                {move || {
                    let user = move || match auth_context.user.get() {
                        Some(Ok(user)) => Some(user),
                        _ => None,
                    };
                    view! {
                        <Show
                            when=move || user().is_some()
                            fallback=|| {
                                view! {
                                    <a href="/register">
                                        Register
                                    </a>
                                    <a href="/login">
                                        Login
                                    </a>
                                }
                            }
                        >

                            <a href="/create_challenge">
                                New Game
                            </a>

                            <a href="/user_account">
                                User Account
                            </a>
                            <a href="/logout">
                                Logout
                            </a>
                        </Show>
                    }
                }}
            </Transition>
            <DarkModeToggle/>
        </header>
    }
}
