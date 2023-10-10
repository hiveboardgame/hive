use leptos::*;
use leptos_meta::*;

use crate::components::organisms::{hamburger::HamburgerDropdown, header::Header};
use crate::providers::color_scheme::ColorScheme;

#[component]
pub fn BaseLayout(children: Children) -> impl IntoView {
    let color_scheme = use_context::<ColorScheme>().expect("Failed to find ColorScheme");
    provide_context(create_rw_signal(HamburgerDropdown(false)));
    view! {
            <Html class=move || {
                let classes = "";
                let theme = match color_scheme.prefers_dark.get() {
                    true => "dark",
                    false => "",
                };
                format!("{} {}", classes, theme)
            }/>
            <Body class="h-screen w-screen bg-white dark:bg-gray-900 max-h-screen max-w-[100vw] overflow-clip"/>
            <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>
            <Header/>
            <main
            class="h-full w-full"
            >
                {children()}
            </main>
    }
}
