use leptos::prelude::*;

const VIDEO_URL: &str = "https://www.youtube.com/watch?v=-_CT8cgOR5Q";
const THUMBNAIL_URL: &str = "https://img.youtube.com/vi/-_CT8cgOR5Q/hqdefault.jpg";

#[component]
pub fn FeaturedVideo() -> impl IntoView {
    view! {
        <div class="w-full m-2 flex flex-col items-center">
            <a
                href=VIDEO_URL
                target="_blank"
                rel="noopener noreferrer"
                class="block w-full max-w-screen-md overflow-hidden rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:border-gray-300 dark:hover:border-gray-600 transition-colors"
                title="An introduction to Hive"
            >
                <div class="p-3 sm:p-4">
                    <h2 class="mb-3 text-lg font-bold text-gray-800 dark:text-gray-200">
                        "Learn to play"
                    </h2>
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
                </div>
            </a>
        </div>
    }
}
