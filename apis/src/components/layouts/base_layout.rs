use leptos::*;
use leptos_meta::*;

use crate::components::organisms::header::Header;
use crate::providers::color_scheme::ColorScheme;

#[component]
pub fn BaseLayout(children: Children) -> impl IntoView {
    let color_scheme = expect_context::<ColorScheme>();
    view! {
        <Html class=move || {
            let classes = "h-screen w-screen max-h-screen max-w-[100vw]";
            let theme = match (color_scheme.prefers_dark)() {
                true => "dark",
                false => "",
            };
            format!("{} {}", classes, theme)
        }/>
        <Body class="h-full w-full bg-white dark:bg-gray-900"/>
        <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>
        <main class="h-full w-full">
            <Header/>
            {children()}
        </main>
    }
}

