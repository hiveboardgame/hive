use leptos::*;
use leptos_meta::*;

use crate::components::organisms::header::Header;
use crate::providers::color_scheme::ColorScheme;

#[component]
pub fn BaseLayout(children: Children) -> impl IntoView {
    let color_scheme = expect_context::<ColorScheme>();
    let color_scheme_meta = move || {
        if (color_scheme.prefers_dark)() {
            "dark".to_string()
        } else {
            "light".to_string()
        }
    };

    view! {
        <Meta name="color-scheme" content=color_scheme_meta/>
        <Html class=move || {
            match (color_scheme.prefers_dark)() {
                true => "dark",
                false => "",
            }
        }/>

        <Body/>
        <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>
        <main class="min-h-screen h-full w-full bg-light dark:bg-dark text-xs sm:text-sm md:text-md lg:text-lg xl-text-xl">
            <Header/>
            {children()}
        </main>
    }
}
