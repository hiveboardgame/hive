use crate::components::bevy::example::BevyExample;
use leptos::prelude::*;

#[component]
pub fn BevyExamplePage() -> impl IntoView {
    view! {
        <div class="pt-20">
            < BevyExample/>
        </div>
    }
}
