use leptos::*;
use crate::organisms::darkmode::DarkModeToggle;

#[component]
pub fn Header(cx: Scope) -> impl IntoView {
    view!{cx,
        <header style="position: sticky; top: 0; display: flex; justify-content: space-between;">
            <a href="/">Home</a>
            <a href="/play">Play</a>
            <DarkModeToggle/>
        </header>
    }
}
