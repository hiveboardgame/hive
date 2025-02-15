use crate::{components::molecules::banner::Banner, providers::ApiRequests};
use leptos::logging::log;
use leptos::*;

#[component]
pub fn Discord() -> impl IntoView {
    let onclick = move |_| {
        let api = ApiRequests::new();
        api.link_discord();
    };

    view! {
        <div class="pt-20">
            <div class="px-4 mx-auto max-w-4xl sm:px-6 lg:px-8">
                <Banner title="Link your Discord account".into_view() />
                <div>
                    <button on:click=onclick>Link Discord</button>
                </div>
            </div>
        </div>
    }
}
