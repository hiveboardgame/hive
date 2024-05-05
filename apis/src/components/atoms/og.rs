use leptos::*;

#[component]
pub fn OG() -> impl IntoView {
    let content = "Free online hive server. Play hive in a clean interface. No ads, no plugin required. Play hive with friends or random opponents.";
    view! {
        <meta name="description" content=content/>

        <meta property="og:url" content="https://hivegame.com"/>
        <meta property="og:type" content="website"/>
        <meta property="og:title" content="The best free, adless Hive server"/>
        <meta property="og:description" content=content/>
        <meta property="og:image" content="https://hivegame.com/assets/android-chrome-512x512.png"/>

        <meta name="twitter:card" content="summary_large_image"/>
        <meta property="twitter:domain" content="hivegame.com"/>
        <meta property="twitter:url" content="https://hivegame.com"/>
        <meta name="twitter:title" content="The best free, adless Hive server"/>
        <meta name="twitter:description" content=content/>
        <meta
            name="twitter:image"
            content="https://hivegame.com/assets/android-chrome-512x512.png"
        />
    }
}
