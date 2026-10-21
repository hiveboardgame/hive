use crate::{providers::tutorial::TutorialContext, tutorial::GuidePage};
use leptos::prelude::*;

#[component]
pub fn OnlinePreview(page: GuidePage) -> impl IntoView {
    let tutorial = expect_context::<TutorialContext>();
    let source = move || {
        let step = tutorial.session.with(|session| session.as_ref().map_or(0, |session| session.step_index()));
        let (screen, focus, anchor) = match page {
            GuidePage::Home => match step.min(2) {
                0 => ("home", "quickplay", "tutorial-quickplay"),
                1 => ("home", "custom", "tutorial-custom-game"),
                _ => ("home", "challenges", "tutorial-open-challenges"),
            },
            GuidePage::Ready => ("home", "feedback", "home-feedback"),
            GuidePage::Settings => ("settings", "", ""),
            GuidePage::Correspondence => ("correspondence", "", ""),
            GuidePage::Tournaments if step == 0 => ("tournaments", "", ""),
            GuidePage::Tournaments => ("tournament-create", "", ""),
            GuidePage::Controls => ("home", "", ""),
        };
        format!("/tutorial/page-preview?screen={screen}&focus={focus}#{anchor}")
    };
    view! {
        <div class="flex flex-col w-full h-full min-h-0">
            <p class="px-3 py-2 text-xs text-gray-600 shrink-0 dark:text-gray-300">
                "Actual site page · read-only · scroll to explore"
            </p>
            <Show when=move || page == GuidePage::Correspondence>
                <div class="flex gap-2 items-center px-3 pb-2 text-sm">
                    <leptos_icons::Icon icon=icondata_ai::AiMailOutlined attr:class="size-6" />
                    "Envelope = correspondence"
                </div>
            </Show>
            <iframe
                title="Actual HiveGame.com page"
                src=source
                sandbox="allow-same-origin"
                tabindex="-1"
                class="block w-full min-h-0 border-0 grow"
            ></iframe>
        </div>
    }
}
