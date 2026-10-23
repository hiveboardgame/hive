use crate::providers::tutorial::TutorialProgress;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn TutorialCard() -> impl IntoView {
    let progress = TutorialProgress::local();
    let href = move || match progress.first_incomplete() {
        Some(id) => format!("/tutorial?lesson={id}"),
        None => "/tutorial".to_string(),
    };

    view! {
        <div class="flex flex-col gap-2 p-4 w-full max-w-xs text-center ui-panel">
            <div class="text-base font-bold text-gray-900 dark:text-gray-100">"New to Hive?"</div>
            <A href=href attr:class="ui-button ui-button-primary ui-button-sm">
                "Tutorial"
            </A>
        </div>
    }
}
