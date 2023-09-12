use leptos::*;
use crate::organisms::header::Header;

#[component]
pub fn Home(cx: Scope) -> impl IntoView {

    view! { cx,
        <Header/>
        Welcome to our Hive
    }
}
