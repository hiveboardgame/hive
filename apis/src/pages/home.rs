use crate::{
    components::{
        atoms::logo::Logo,
        molecules::{online_users::OnlineUsers, rl_banner::RlBanner},
        organisms::{calendar::Calendar, challenges::Challenges, quickplay::QuickPlay, tv::Tv},
    },
    functions::home_banner,
    providers::RealtimeEnabledContext,
};
use leptos::prelude::*;

#[component]
pub fn Home() -> impl IntoView {
    let banner = OnceResource::new(async move { home_banner::get().await.ok().flatten() });
    let realtime_ctx = expect_context::<RealtimeEnabledContext>();
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
            <div class="grid grid-cols-1 gap-6 items-start px-4 mx-auto w-full max-w-screen-xl 2xl:max-w-screen-2xl lg:grid-cols-[minmax(18rem,20rem)_minmax(0,1fr)] 2xl:grid-cols-[minmax(18rem,20rem)_minmax(0,1fr)_minmax(18rem,20rem)]">
                <div class="contents lg:flex lg:flex-col lg:col-start-1 lg:row-start-1 lg:items-center lg:space-y-4 lg:min-w-0">
                    <div class="flex flex-col order-1 items-center min-w-0 lg:order-none">
                        <Logo tw_class="flex w-48 lg:w-72" />
                    </div>
                    <div class="order-4 mx-auto mt-4 w-full min-w-0 max-w-md lg:overflow-y-auto lg:order-none lg:mt-0 lg:max-h-[50rem]">
                        <Calendar />
                    </div>
                </div>
                <div class="flex flex-col order-2 items-center space-y-6 min-w-0 lg:order-none lg:col-start-2 lg:row-start-1">
                    <QuickPlay realtime_enabled=Signal::derive(move || realtime_ctx.0.get()) />
                    <Challenges realtime_disabled=Signal::derive(move || !realtime_ctx.0.get()) />
                    <div class="w-full lg:flex lg:justify-end 2xl:justify-center">
                        <div class="w-full lg:max-w-screen-md">
                            <div class="w-full lg:flow-root">
                                <div class="hidden float-right lg:block lg:ml-6 2xl:hidden">
                                    <OnlineUsers />
                                </div>
                                <Tv />
                            </div>
                        </div>
                    </div>
                </div>
                <div class="flex flex-col order-3 items-center min-w-0 lg:hidden 2xl:flex 2xl:order-none 2xl:col-start-3 2xl:row-start-1">
                    <OnlineUsers />
                </div>
            </div>
        </div>
    }
}
