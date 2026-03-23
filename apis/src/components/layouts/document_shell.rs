use leptos::prelude::*;
use leptos_meta::{HashedStylesheet, MetaTags};

#[component]
pub fn DocumentShell(
    leptos_options: LeptosOptions,
    manifest_href: String,
    apple_touch_icon_href: String,
    pwa_script_src: String,
    children: ChildrenFn,
) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta
                    name="viewport"
                    content="width=device-width, initial-scale=1, interactive-widget=resizes-content, user-scalable=no"
                />
                <link rel="manifest" href=manifest_href />
                <link rel="apple-touch-icon" href=apple_touch_icon_href />
                <meta name="mobile-web-app-capable" content="yes" />
                <meta name="apple-mobile-web-app-status-bar-style" content="black" />
                <script src=pwa_script_src></script>
                <AutoReload options=leptos_options.clone() />
                <HydrationScripts options=leptos_options.clone() />
                <MetaTags />
                <HashedStylesheet options=leptos_options id="leptos" />
            </head>
            <body>{children()}</body>
        </html>
    }
}
