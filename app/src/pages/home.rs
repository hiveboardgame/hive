use leptos::*;
use crate::organisms::header::Header;

#[component]
pub fn Home() -> impl IntoView {

    view! {
        <Header/>
        Welcome to our Hive
    }
}
