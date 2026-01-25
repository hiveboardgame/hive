use crate::components::atoms::logo::Logo;
use crate::components::molecules::online_users::OnlineUsers;
use crate::components::molecules::rl_banner::RlBanner;
use crate::components::organisms::{
    calendar::Calendar, challenges::Challenges, quickplay::QuickPlay, tv::Tv,
};
use crate::functions::home_banner;
use leptos::prelude::*;

#[component]
pub fn Home() -> impl IntoView {
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
            <div class="grid w-full max-w-screen-xl 2xl:max-w-screen-2xl grid-cols-1 gap-6 items-start px-4 mx-auto lg:grid-cols-[minmax(18rem,20rem)_minmax(0,1fr)] 2xl:grid-cols-[minmax(18rem,20rem)_minmax(0,1fr)_minmax(18rem,20rem)]">
                <div class="contents lg:flex lg:flex-col lg:items-center lg:space-y-4 lg:min-w-0 lg:col-start-1 lg:row-start-1">
                    <div class="order-1 flex flex-col items-center min-w-0 lg:order-none">
                        <Logo tw_class="flex w-48 lg:w-72" />
                    </div>
                    <div class="order-4 w-full max-w-md min-w-0 mx-auto mt-4 lg:order-none lg:mt-0 lg:max-h-[50rem] lg:overflow-y-auto">
                        <Calendar />
                    </div>
                </div>
                <div class="flex flex-col items-center space-y-6 min-w-0 order-2 lg:order-none lg:col-start-2 lg:row-start-1">
                    <QuickPlay />
                    <Challenges />
                    <div class="w-full lg:flex lg:justify-end 2xl:justify-center">
                        <div class="w-full lg:max-w-screen-md">
                            <div class="w-full lg:flow-root">
                                <div class="hidden lg:block 2xl:hidden float-right lg:ml-6">
                                    <OnlineUsers />
                                </div>
                                <Tv />
                            </div>
                        </div>
                    </div>
                </div>
                <div class="flex flex-col items-center min-w-0 order-3 lg:hidden 2xl:flex 2xl:order-none 2xl:col-start-3 2xl:row-start-1">
                    <OnlineUsers />
                </div>
            </div>
        </div>
    }
}
