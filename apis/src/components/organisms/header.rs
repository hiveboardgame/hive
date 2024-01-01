use crate::components::atoms::next_game_button::NextGameButton;
use crate::components::organisms::{
    darkmode_toggle::DarkModeToggle, hamburger::Hamburger, logout::Logout,
};
use crate::providers::auth_context::*;
use leptos::*;
use leptos_router::use_location;

#[derive(Clone)]
pub struct Redirect(pub RwSignal<String>);

#[component]
pub fn Header(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let hamburger_show = create_rw_signal(false);
    let onclick = move || hamburger_show.update(|b| *b = false);
    view! {
        <header class=format!(
            "h-8 md:h-10 w-full fixed top-0 flex justify-between items-center bg-gray-300 dark:bg-gray-700 z-50 max-w-[100vw] {extend_tw_classes}",
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
                                        <a
                                            href="/login"
                                            on:focus=move |_| set_redirect()
                                            on:click=onclick
                                        >

                                            Login
                                        </a>
                                        <Hamburger hamburger_show=hamburger_show>
                                            <ul>
                                                <a
                                                    href="/register"
                                                    on:focus=move |_| set_redirect()
                                                    on:click=onclick
                                                >
                                                    Register
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

                            <div>
                                <NextGameButton/>
                            </div>

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
                                    <a
                                        href="/account"
                                        on:focus=move |_| set_redirect()
                                        on:click=move |_| onclick()
                                    >
                                        Edit Account
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

fn set_redirect() {
    let referrer = RwSignal::new(String::from("/"));
    let location = use_location().pathname.get();
    referrer.update(|s| *s = location);
    provide_context(Redirect(referrer));
}
