use crate::components::organisms::lobby::Lobby;
use leptos::*;

#[component]
pub fn Home(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    view! {
        <div class=format!("{extend_tw_classes}")>
            <Lobby/>
        </div>
    }
}
