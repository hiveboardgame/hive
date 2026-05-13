use crate::{i18n::*, providers::Config};
use leptos::prelude::*;
use leptos_icons::Icon;

const VIDEO_URL: &str = "https://www.youtube.com/watch?v=-_CT8cgOR5Q";
const THUMBNAIL_URL: &str = "/assets/featured_video_thumbnail.jpg";

#[component]
pub fn FeaturedVideo() -> impl IntoView {
    let Config(config, set_cookie) = expect_context();

    let i18n = use_i18n();

    // Cookie state is synchronous so we only depend on that for visibility.
    let show = move || !config().video_dismissed;

    view! {
        <Show when=show>
            <div class="flex overflow-hidden justify-center m-2 w-full lg:justify-end 2xl:justify-center">
                <div class="overflow-hidden w-full max-w-screen-md bg-white rounded-lg border border-gray-200 transition-colors dark:bg-gray-800 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600">
                    <div class="flex justify-between items-center px-3 pt-3 mb-3 sm:px-4 sm:pt-4">
                        <h2 class="text-lg font-bold text-gray-800 dark:text-gray-200">
                            {t!(i18n, home.learn_to_play)}
                        </h2>
                        <button
                            on:click=move |_| {
                                set_cookie
                                    .update(|c| {
                                        if let Some(cookie) = c {
                                            cookie.video_dismissed = true;
                                        }
                                    });
                            }
                            class="flex justify-center items-center text-white rounded-full shadow transition-colors hover:bg-red-700 size-8 bg-ladybug-red shrink-0"
                            aria-label="Dismiss"
                        >
                            <Icon icon=icondata_io::IoCloseSharp attr:class="size-5" />
                        </button>
                    </div>
                    <a
                        href=VIDEO_URL
                        target="_blank"
                        rel="noopener noreferrer"
                        class="block px-3 pb-3 w-full no-underline sm:px-4 sm:pb-4"
                        title="An introduction to Hive"
                    >
                        <div class="overflow-hidden relative w-full bg-black rounded aspect-video">
                            <img
                                src=THUMBNAIL_URL
                                alt="Learn to play Hive - video thumbnail"
                                class="object-cover w-full h-full"
                                loading="lazy"
                            />
                            <div class="flex absolute inset-0 justify-center items-center transition-colors bg-black/20 hover:bg-black/30">
                                <div class="flex justify-center items-center text-white rounded-full shadow-lg size-16 bg-red-600/90 sm:size-20">
                                    <svg
                                        class="ml-1 size-8 sm:size-10"
                                        viewBox="0 0 24 24"
                                        fill="currentColor"
                                        aria-hidden="true"
                                    >
                                        <path d="M8 5v14l11-7z" />
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
