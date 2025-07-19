use crate::components::atoms::logo::Logo;
use crate::components::molecules::online_users::OnlineUsers;
use crate::components::molecules::rl_banner::RlBanner;
use crate::components::organisms::{
    calendar::Calendar, challenges::Challenges, quickplay::QuickPlay, tv::Tv,
};
use crate::functions::home_banner;
use leptos::prelude::*;
use leptos_use::use_media_query;

#[component]
pub fn Home() -> impl IntoView {
    let in_column = use_media_query("(max-width: 1023px)");
    let banner = OnceResource::new(async move { home_banner::get().await.ok().flatten() });
    view! {
        <div class="flex overflow-x-hidden flex-col justify-start items-center pt-20 w-full md:justify-center">
            <Transition>
                {move || {
                    banner
                        .get()
                        .flatten()
                        .map(|banner| {
                            view! {
                                <div>
                                    <RlBanner title=banner.title content=banner.content />
                                </div>
                            }
                        })
                }}
            </Transition>
            <Show when=in_column>
                <div class="container flex flex-col justify-center items-center">
                    <Logo tw_class="flex w-48" />
                    <div class="flex flex-col items-center w-full">
                        <QuickPlay />
                        <Challenges />
                    </div>
                    <Tv />
                    <OnlineUsers />
                    <div class="mt-4 w-full max-w-md">
                        <Calendar />
                    </div>
                </div>
            </Show>
            <Show when=move || !in_column()>
                <div class="container grid grid-cols-[300px_1fr_300px] gap-6 items-start">
                    <div class="flex flex-col items-center space-y-4">
                        <Logo tw_class="flex w-72" />
                        <div class="w-full max-w-md max-h-[50rem] overflow-y-auto">
                            <Calendar />
                        </div>
                    </div>

                    <div class="flex flex-col items-center space-y-6">
                        <QuickPlay />
                        <Challenges />
                        <Tv />
                    </div>

                    <div class="flex flex-col items-center">
                        <OnlineUsers />
                    </div>
                </div>
            </Show>
        </div>
    }
}
