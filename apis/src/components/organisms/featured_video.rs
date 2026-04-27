use crate::{i18n::*, providers::{AuthContext, Config}};
use leptos::prelude::*;

const VIDEO_URL: &str = "https://www.youtube.com/watch?v=-_CT8cgOR5Q";
const THUMBNAIL_URL: &str = "https://img.youtube.com/vi/-_CT8cgOR5Q/hqdefault.jpg";

#[component]
pub fn FeaturedVideo() -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let Config(config, set_cookie) = expect_context();

    let i18n = use_i18n();
    let is_logged_in = move || auth.user.with(|u: &Option<_>| u.is_some());

    // auth.user is async (derived from a server action), so is_logged_in()
    // starts as false and resolves later. Gating `show` on it causes a flash:
    // dismissed users briefly appear logged-out, which forces show=true.
    // Cookie state is synchronous so we only depend on that for visibility.
    let show = move || !config().video_dismissed;

    view! {
        <Show when=show>
            <div class="w-full m-2 flex flex-col items-center">
                <div class="w-full max-w-screen-md overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:border-gray-300 dark:hover:border-gray-600 transition-colors">
                    <div class="flex items-center justify-between px-3 pt-3 mb-3 sm:px-4 sm:pt-4">
                        <h2 class="text-lg font-bold text-gray-800 dark:text-gray-200">
                            {t!(i18n, home.learn_to_play)}
                        </h2>
                        {move || is_logged_in().then(|| view! {
                            <button
                                on:click=move |_| {
                                    set_cookie.update(|c| {
                                        if let Some(cookie) = c {
                                            cookie.video_dismissed = true;
                                        }
                                    });
                                }
                                class="flex items-center justify-center size-8 rounded-full bg-ladybug-red text-white hover:bg-red-700 transition-colors shadow shrink-0"
                                aria-label="Dismiss"
                            >
                                "✕"
                            </button>
                        })}
                    </div>
                    <a
                        href=VIDEO_URL
                        target="_blank"
                        rel="noopener noreferrer"
                        class="block w-full px-3 pb-3 sm:px-4 sm:pb-4 no-underline"
                        title="An introduction to Hive"
                    >
                        <div class="relative w-full aspect-video rounded overflow-hidden bg-black">
                            <img
                                src=THUMBNAIL_URL
                                alt="Learn to play Hive - video thumbnail"
                                class="w-full h-full object-cover"
                                loading="lazy"
                            />
                            <div class="absolute inset-0 flex items-center justify-center bg-black/20 hover:bg-black/30 transition-colors">
                                <div class="flex size-16 items-center justify-center rounded-full bg-red-600/90 text-white shadow-lg sm:size-20">
                                    <svg class="ml-1 size-8 sm:size-10" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                                        <path d="M8 5v14l11-7z"/>
                                    </svg>
                                </div>
                            </div>
                        </div>
                    </a>
                </div>
            </div>
        </Show>
    }
}
