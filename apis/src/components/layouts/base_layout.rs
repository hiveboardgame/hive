use leptos::*;
use leptos_meta::*;

use crate::components::organisms::header::Header;
use crate::providers::color_scheme::ColorScheme;

#[component]
pub fn BaseLayout(children: Children) -> impl IntoView {
    let color_scheme = expect_context::<ColorScheme>();
    view! {
        <Html class=move || {
            match (color_scheme.prefers_dark)() {
                true => "dark",
                false => "",
            }
        }/>

        <Body/>
        <Stylesheet id="leptos" href="/pkg/HiveGame.css"/>
        <main class="min-h-screen h-full w-full bg-white dark:bg-gray-900">
            <Header/>
            {children()}
        </main>
    }
}

