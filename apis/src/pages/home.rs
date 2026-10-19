use crate::{
    components::{
        atoms::logo::Logo,
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::{online_users::OnlineUsers, rl_banner::RlBanner},
        organisms::{
            calendar::Calendar,
            challenges::Challenges,
            featured_video::FeaturedVideo,
            live_arenas::LiveArenas,
            quickplay::QuickPlay,
            recent_evals::RecentEvals,
            tv::Tv,
        },
    },
    functions::home_banner,
    hooks::recent_evals::use_recent_evals,
    providers::game_eval::evals_visible,
};
use leptos::prelude::*;

#[component]
pub fn Home() -> impl IntoView {
    let evals_visible = evals_visible();
    let recent_evals = use_recent_evals(evals_visible);
    let banner = OnceResource::new(async move { home_banner::get().await.ok().flatten() });
    view! {
        <PageShell variant=PageShellVariant::Dashboard class="overflow-x-hidden">
            <Transition>
                {move || {
                    banner
                        .get()
                        .flatten()
                        .map(|banner| {
                            view! {
                                <div class="w-full lg:px-4">
                                    <div class="mx-auto w-full max-w-4xl">
                                        <RlBanner title=banner.title content=banner.content />
                                    </div>
                                </div>
                            }
                        })
                }}
            </Transition>
            <div class="ui-home-grid">
                <div class="contents lg:flex lg:flex-col lg:col-start-1 lg:row-start-1 lg:items-center lg:space-y-4 lg:min-w-0">
                    <div class="flex flex-col order-1 items-center w-full min-w-0 lg:order-none">
                        <Logo tw_class="flex w-48 lg:w-[90%]" />
                    </div>
                    <div class="order-4 mx-auto w-full min-w-0 max-w-md lg:overflow-y-auto lg:order-none lg:mt-0 lg:max-h-[50rem]">
                        <Calendar />
                    </div>
                </div>
                <div class="flex flex-col order-2 gap-3 items-center min-w-0 sm:gap-6 lg:order-none lg:col-start-2 lg:row-start-1">
                    <div class="contents [&>*]:order-2 lg:[&>*]:order-1">
                        <FeaturedVideo />
                    </div>
                    <div class="order-1 w-full lg:order-2">
                        <QuickPlay />
                    </div>
                    <div class="contents [&>*]:order-2">
                        <LiveArenas />
                    </div>
                    <div class="order-3 w-full">
                        <Challenges />
                    </div>
                    <div class="order-4 w-full">
                        <div class="mx-auto w-full">
                            <div class="w-full lg:flow-root">
                                <div class="hidden float-right w-64 lg:flex lg:flex-col lg:gap-4 lg:ml-6 2xl:hidden">
                                    <OnlineUsers />
                                    <Show when=move || evals_visible.get()>
                                        <RecentEvals evals=recent_evals />
                                    </Show>
                                </div>
                                <Tv />
                            </div>
                        </div>
                    </div>
                </div>
                <div class="flex flex-col order-3 gap-4 mx-auto w-full min-w-0 max-w-md lg:hidden 2xl:flex 2xl:order-none 2xl:col-start-3 2xl:row-start-1 2xl:mx-0 2xl:max-w-none">
                    <OnlineUsers />
                    <Show when=move || evals_visible.get()>
                        <RecentEvals evals=recent_evals />
                    </Show>
                </div>
            </div>
        </PageShell>
    }
}
