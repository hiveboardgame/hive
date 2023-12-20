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
        <header class=format!(
            "w-full sticky top-0 flex justify-between items-center bg-gray-300 dark:bg-gray-700 z-50 max-w-[100vw] {extend_tw_classes}",
        )>
            <a class="ml-10" href="/">
                Home
            </a>
            <Transition>
                {move || {
                    let user = move || match (auth_context.user)() {
                        Some(Ok(Some(user))) => Some(user),
                        _ => None,
                    };
                    view! {
                        <Show
                            when=move || user().is_some()
                            fallback=|| {
                                let hamburger_show = create_rw_signal(false);
                                let onclick = move |_| hamburger_show.update(|b| *b = false);
                                view! {
                                    <div>
                                        <a href="/login" on:click=onclick>
                                            Login
                                        </a>
                                        <Hamburger hamburger_show=hamburger_show>

                                            <ul>
                                                <a href="/register" on:click=onclick>
                                                    Register
                                                </a>
                                            </ul>
                                            <ul>
                                                <a href="/hws" on:click=onclick>
                                                    WebSocket
                                                </a>
                                            </ul>
                                            <ul>
                                                <DarkModeToggle/>
                                            </ul>
                                        </Hamburger>
                                    </div>
                                }
                            }
                        >

                            <Hamburger hamburger_show=hamburger_show>
                                <ul>
                                    <a
                                        href=format!(
                                            "/@/{}",
                                            user().expect("User is some").username,
                                        )

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
                                    <a href="/hws" on:click=move |_| onclick()>
                                        WebSocket
                                    </a>
                                </ul>
                                <ul>
                                    <DarkModeToggle/>
                                </ul>
                                <ul>
                                    <Logout on:submit=move |_| onclick()/>
                                </ul>

                            </Hamburger>
                        </Show>
                    }
                }}

            </Transition>

        </header>
    }
}
