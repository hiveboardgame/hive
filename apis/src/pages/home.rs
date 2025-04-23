use crate::components::atoms::logo::Logo;
use crate::components::molecules::rl_banner::RlBanner;
use crate::components::organisms::{
    challenges::Challenges, online_users::OnlineUsers, quickplay::QuickPlay, tv::Tv,
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
            <div class="container flex flex-col justify-center items-center lg:flex-row lg:items-start">
                <div class="flex justify-center items-center">
                    <Logo tw_class="flex lg:w-72 w-48" />
                </div>
                <div class="flex flex-col justify-center items-center w-full md:flex-row">
                    <div class="flex flex-col items-center basis-2/3">
                        <div class="flex flex-col justify-center items-center">
                            <Challenges />
                            <QuickPlay />
                        </div>
                    </div>
                </div>
                <Show when=in_column>
                    <Tv />
                </Show>
                <OnlineUsers />
            </div>
            <Show when=move || !in_column()>
                <Tv />
            </Show>
        </div>
    }
}
