use crate::components::atoms::logo::Logo;
use crate::components::organisms::{
    challenges::Challenges, online_users::OnlineUsers, quickplay::QuickPlay, tv::Tv,
};
use leptos::*;

#[component]
pub fn Home() -> impl IntoView {
    view! {
        <div class="flex overflow-x-hidden flex-col justify-start items-center pt-20 w-full md:justify-center">
            <div class="flex flex-col justify-center items-center lg:flex-row lg:items-start">
                <Logo tw_class="flex w-48 lg:w-72"/>
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
