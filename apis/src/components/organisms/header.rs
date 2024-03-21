use crate::components::atoms::next_game_button::NextGameButton;
use crate::components::layouts::base_layout::DROPDOWN_BUTTON_STYLE;
use crate::components::organisms::{
    darkmode_toggle::DarkModeToggle,
    dropdowns::{
        CommunityDropdown, LearnDropdown, MobileDropdown, TournamentDropdown, UserDropdown,
    },
};
use crate::providers::auth_context::*;
use leptos::*;
use leptos_router::use_location;
use shared_types::time_mode::TimeMode;

#[derive(Clone)]
pub struct Redirect(pub RwSignal<String>);

#[component]
pub fn Header(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    view! {
        <header class=format!(
            "w-full fixed top-0 flex justify-between items-center bg-gray-300 dark:bg-gray-700 z-50 max-w-[100vw] {extend_tw_classes}",
        )>
            <Transition fallback=|| {
                view! {
                    <a class="lg:ml-10" href="/">
                        HiveGame.com
                    </a>
                    <div class="flex items-center lg:mr-10">
                        <DarkModeToggle/>
                        <a
                            class="bg-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 m-1 px-4 rounded"
                            href="/login"
                            on:focus=move |_| set_redirect()
                        >

                            Login
                        </a>
                    </div>
                }
            }>
                {move || {
                    let user = move || match (auth_context.user)() {
                        Some(Ok(Some(user))) => Some(user),
                        _ => None,
                    };
                    view! {
                        <Show
                            when=move || user().is_some()
                            fallback=|| {
                                view! {
                                    <a class="lg:ml-10" href="/">
                                        HiveGame.com
                                    </a>
                                    <div class="flex items-center lg:mr-10">
                                        <DarkModeToggle/>
                                        <a
                                            class="bg-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 m-1 px-4 rounded"
                                            href="/login"
                                            on:focus=move |_| set_redirect()
                                        >

                                            Login
                                        </a>
                                    </div>
                                }
                            }
                        >

                            <div class="lg:ml-10 flex gap-1 items-center">
                                <MobileDropdown/>

                                <a class="hidden md:block m-2" href="/">
                                    HiveGame.com
                                </a>
                                <div class="hidden lg:flex lg:items-center lg:gap-1">
                                    <LearnDropdown/>
                                    <TournamentDropdown/>
                                    <CommunityDropdown/>
                                    <a class=DROPDOWN_BUTTON_STYLE href="https://www.gen42.com/">
                                        Get Game
                                    </a>
                                    <a class=DROPDOWN_BUTTON_STYLE href="/donate">
                                        Donate
                                    </a>
                                </div>
                            </div>
                            <div class="flex items-center">
                                <NextGameButton time_mode=store_value(TimeMode::RealTime)/>
                                <NextGameButton time_mode=store_value(TimeMode::Correspondence)/>
                                <NextGameButton time_mode=store_value(TimeMode::Untimed)/>
                            </div>
                            <div class="flex items-center lg:mr-10">
                                <DarkModeToggle/>
                                <UserDropdown username=user().expect("User is some").username/>
                            </div>
                        </Show>
                    }
                }}

            </Transition>
        </header>
    }
}

pub fn set_redirect() {
    let referrer = RwSignal::new(String::from("/"));
    let location = use_location().pathname.get();
    referrer.update(|s| *s = location);
    provide_context(Redirect(referrer));
}
