use crate::{common::with_class, i18n::*, providers::Config};
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
            <div class=with_class(
                "ui-panel",
                "overflow-hidden mx-auto w-full max-w-screen-md transition-colors hover:border-pillbug-teal/40",
            )>
                <div class="ui-panel-header">
                    <h2 class="text-lg font-bold text-gray-900 dark:text-gray-100">
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
                        class="ui-button ui-button-danger ui-button-icon shrink-0"
                        aria-label="Dismiss"
                    >
                        <Icon icon=icondata_io::IoCloseSharp attr:class="size-5" />
                    </button>
                </div>
                <a
                    href=VIDEO_URL
                    target="_blank"
                    rel="noopener noreferrer"
                    class="block pt-3 w-full no-underline ui-panel-body group"
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
                            <div class="flex justify-center items-center text-white rounded-full ring-1 shadow-lg transition-colors bg-ladybug-red/90 ring-white/40 size-16 sm:size-20 group-hover:bg-ladybug-red">
                                <Icon
                                    icon=icondata_fa::FaPlaySolid
                                    attr:class="ml-1 size-8 fill-current sm:size-10"
                                />
                            </div>
                        </div>
                    </div>
                </a>
            </div>
        </Show>
    }
}
