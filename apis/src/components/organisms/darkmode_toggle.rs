use crate::providers::color_scheme::ColorScheme;
use leptos::*;
use leptos_router::ActionForm;

#[component]
pub fn DarkModeToggle() -> impl IntoView {
    let color_scheme = expect_context::<ColorScheme>();

    view! {
        <ActionForm action=color_scheme.action>
            <input
                type="hidden"
                name="prefers_dark"
                value=move || (!(color_scheme.prefers_dark)()).to_string()
            />
            <button
                type="submit"
                class="inline-flex items-center m-1 px-4 py-2 border border-transparent text-base font-medium rounded-md shadow mx-4 dark:bg-yellow-400 bg-gray-700"
                value=move || { if (color_scheme.prefers_dark)() { "dark" } else { "light" } }
                inner_html=move || {
                    if (color_scheme.prefers_dark)() {
                        r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-gray-700 bg-yellow-400 dark:text-text-gray-700" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                                <path strokeLinecap="round" strokeLinejoin="round" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" />
                            </svg>
                        "#
                    } else {
                        r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-yellow-400 bg-gray-700" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth="2">
                                <path strokeLinecap="round" strokeLinejoin="round" d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z" />
                            </svg>
                        "#
                    }
                }
            >
            </button>
        </ActionForm>
    }
}

