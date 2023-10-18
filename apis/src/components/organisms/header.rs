use crate::components::organisms::{
    darkmode_toggle::DarkModeToggle, hamburger::Hamburger, logout::Logout,
};
use crate::providers::auth_context::*;
use leptos::*;

#[component]
pub fn Header(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let hamburger_show = create_rw_signal(false);
    let onclick = move || hamburger_show.update(|b| *b = false);
    view! {
        <header class=format!("w-full sticky top-0 flex justify-between {extend_tw_classes}")>
            <a href="/">Home</a>
            <a href="/hws">WebSocket</a>
            <Transition>
                {move || {
                    let user = move || match auth_context.user.get() {
                        Some(Ok(Some(user))) => Some(user),
                        _ => None,
                    };
                    view! {
                        <Show
                            when=move || user().is_some()
                            fallback=|| {
                                view! {
                                    <a href="/register">Register</a>
                                    <a href="/login">Login</a>
                                }
                            }
                        >

                            <Hamburger hamburger_show=hamburger_show>
                                <ul>
                                    <a
                                        href=format!("/@/{}", user().unwrap().username)
                                        on:click=move |_| onclick()
                                    >
                                        Profile
                                    </a>
                                </ul>
                                <ul>
                                    <a href="/account" on:click=move |_| onclick()>
                                        Edit Account
                                    </a>
                                </ul>
                                <ul>
                                    <Logout on:submit=move |_| onclick()/>
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

