use crate::providers::tutorial::TutorialProgress;
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn TutorialCard() -> impl IntoView {
    let progress = TutorialProgress::local();
    let started = move || progress.completed_count() > 0;
    let href = move || match progress.first_incomplete() {
        Some(id) => format!("/tutorial?lesson={id}"),
        None => "/tutorial".to_string(),
    };
    let label = move || {
        if !started() {
            "Start the tutorial".to_string()
        } else if progress.first_incomplete().is_none() {
            "Review the tutorial".to_string()
        } else {
            format!(
                "Continue ({}/{})",
                progress.completed_count(),
                progress.total()
            )
        }
    };

    view! {
        <div class="flex flex-col gap-2 p-4 w-full max-w-xs text-center ui-panel">
            <div class="text-base font-bold text-gray-900 dark:text-gray-100">"New to Hive?"</div>
            <p class="text-sm text-gray-600 dark:text-gray-300">
                "Learn every piece by playing it, then move on to strategy and real tactics."
            </p>
            <A href=href attr:class="ui-button ui-button-primary ui-button-sm">
                {label}
            </A>
        </div>
    }
}
