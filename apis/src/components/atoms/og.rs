use leptos::prelude::*;
use leptos_meta::Meta;
#[component]
pub fn OG() -> impl IntoView {
    let content = "Free online hive server. Play hive in a clean interface. No ads, no plugin required. Play hive with friends or random opponents.";
    view! {
        <Meta name="description" content=content />

        <Meta property="og:url" content="https://hivegame.com" />
        <Meta property="og:type" content="website" />
        <Meta property="og:title" content="The best free, adless Hive server" />
        <Meta property="og:description" content=content />
        <Meta property="og:image" content="https://hivegame.com/assets/stacked_3D.png" />

        <Meta name="twitter:card" content="summary_large_image" />
        <Meta property="twitter:domain" content="hivegame.com" />
        <Meta property="twitter:url" content="https://hivegame.com" />
        <Meta name="twitter:title" content="The best free, adless Hive server" />
        <Meta name="twitter:description" content=content />
        <Meta name="twitter:image" content="summary_large_image" />
    }
}
