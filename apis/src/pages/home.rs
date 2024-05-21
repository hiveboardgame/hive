use crate::components::molecules::banner::Banner;
use crate::components::organisms::challenges::Challenges;
use crate::components::organisms::online_users::OnlineUsers;
use crate::components::organisms::quickplay::QuickPlay;
use crate::components::organisms::tv::Tv;
use leptos::*;

#[component]
pub fn Home() -> impl IntoView {
    let logo = move || {
        view! { <img width="100%" height="100%" src="/assets/favicon-32x32.png" alt="ladybug"/> }
    };

    view! {
        <div class="flex overflow-x-hidden flex-col justify-start items-center pt-20 w-full md:justify-center">
            <Banner title="hivegame.com" extend_tw_classes="w-10/12" logo=logo()/>
            <div class="flex flex-col justify-center items-center lg:flex-row lg:items-start">
                <div class="flex flex-col items-center w-full md:flex-row">
                    <div class="flex flex-col items-center">
                        <div class="flex flex-col items-center sm:w-[500px] lg:w-[550px]">
                            <Challenges/>
                            <QuickPlay/>
                        </div>
                        <Tv/>
                    </div>
                </div>
                <OnlineUsers/>
            </div>
        </div>
    }
}
