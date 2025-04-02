use crate::providers::Config;
use leptos::prelude::*;

#[component]
pub fn DarkModeToggle(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let Config(config, set_cookie) = expect_context();
    view! {
        <div class=format!(
            "inline-flex justify-center items-center m-1 rounded dark:bg-orange-twilight bg-gray-950 {extend_tw_classes}",
        )>

            <button
                on:click=move |_| {
                    set_cookie
                        .update(|c| {
                            if let Some(cookie) = c {
                                cookie.prefers_dark = !cookie.prefers_dark;
                            }
                        });
                }

                class="flex justify-center items-center px-1 py-2 w-full h-full"
                value=move || { if config().prefers_dark { "dark" } else { "light" } }
                inner_html=move || {
                    if config().prefers_dark {
                        r#"<svg xmlns="http://www.w3.org/2000/svg" class="w-6 h-4 text-hive-black bg-orange-twilight dark:text-text-gray-700" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                                <path strokeLinecap="round" strokeLinejoin="round" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                            </svg>
                        "#
                    } else {
                        r#"<svg xmlns="http://www.w3.org/2000/svg" class="w-6 h-4 text-orange-twilight bg-gray-950" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                                <path strokeLinecap="round" strokeLinejoin="round" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                            </svg>
                        "#
                    }
                }
            ></button>
        </div>
    }
}
