use crate::components::organisms::{
    darkmode_toggle::DarkModeToggle,
    hamburger::{Hamburger, HamburgerDropdown},
    logout::Logout,
};
use crate::providers::auth_context::*;
use leptos::*;

#[component]
pub fn Header(#[prop(default = "")] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = use_context::<AuthContext>().expect("Failed to get AuthContext");
    let visible = use_context::<RwSignal<HamburgerDropdown>>().expect("An open/closed context");
    let onclick = move |_| visible.update(|b| *b = HamburgerDropdown(false));
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

                            <Hamburger fallback= move || ()>
                                <ul>
                                    <a href="/create_challenge" on:click=onclick>
                                        New Game
                                    </a>
                                </ul>
                                <ul>
                                    <a href="/user_account" on:click=onclick>
                                        User Account
                                    </a>
                                </ul>
                                <ul>
                                    <Logout />
                                </ul>
                            </Hamburger>

                        </Show>
                    }
                }}
            </Transition>
            <DarkModeToggle/>
        </header>
    }
}
