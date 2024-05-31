use crate::components::atoms::next_game_button::NextGameButton;
use crate::components::molecules::chat_and_controls::ChatAndControls;
use crate::components::organisms::{
    darkmode_toggle::DarkModeToggle,
    dropdowns::{
        CommunityDropdown, LearnDropdown, MobileDropdown, TournamentDropdown, UserDropdown,
    },
};
use crate::providers::AuthContext;
use leptos::*;
use leptos_router::use_location;
use shared_types::TimeMode;

#[derive(Clone)]
pub struct Redirect(pub RwSignal<String>);

#[component]
pub fn Header() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();

    view! {
        <header class="w-full fixed top-0 flex justify-between items-center bg-gray-300 dark:bg-header-twilight z-50 max-w-[100vw] select-none">
            <Transition fallback=|| {
                view! {
                    <div class="flex gap-1 items-center lg:ml-10">
                        <MobileDropdown/>

                        <a class="hidden m-2 md:block" href="/">
                            Home
                        </a>
                        <div class="hidden lg:flex lg:items-center lg:gap-1">
                            <CommunityDropdown/>
                            <LearnDropdown/>
                            <TournamentDropdown/>
                            <a
                                class="block p-2 h-full whitespace-nowrap transition-transform duration-300 transform hover:text-pillbug-teal active:scale-95"
                                href="https://www.gen42.com/"
                                rel="external"
                                target="_blank"
                            >
                                Buy Game
                            </a>
                            <a
                                class="block p-2 h-full whitespace-nowrap transition-transform duration-300 transform text-orange-twilight hover:text-pillbug-teal active:scale-95"
                                href="/donate"
                            >
                                Donate
                            </a>
                        </div>
                    </div>
                    <div class="flex items-center lg:mr-10">
                        <ChatAndControls/>
                        <DarkModeToggle extend_tw_classes="max-h-6 sm:max-h-7"/>
                        <a
                            class="px-4 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
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
                        <div class="flex gap-1 items-center lg:ml-10">
                            <MobileDropdown/>

                            <a class="hidden m-2 md:block" href="/">
                                Home
                            </a>
                            <div class="hidden lg:flex lg:items-center lg:gap-1">
                                <CommunityDropdown/>
                                <LearnDropdown/>
                                <TournamentDropdown/>

                                <a
                                    class="block p-2 h-full whitespace-nowrap transition-transform duration-300 transform hover:text-pillbug-teal active:scale-95"
                                    href="https://www.gen42.com/"
                                    rel="external"
                                    target="_blank"
                                >
                                    Buy Game
                                </a>
                                <a
                                    class="block p-2 h-full whitespace-nowrap transition-transform duration-300 transform text-orange-twilight hover:text-pillbug-teal active:scale-95"
                                    href="/donate"
                                >
                                    Donate
                                </a>
                            </div>
                        </div>
                        <Show
                            when=move || user().is_some()
                            fallback=|| {
                                view! {
                                    <div class="flex items-center lg:mr-10">
                                        <ChatAndControls/>
                                        <DarkModeToggle extend_tw_classes="max-h-6 sm:max-h-7"/>
                                        <a
                                            class="px-4 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
                                            href="/login"
                                            on:focus=move |_| set_redirect()
                                        >

                                            Login
                                        </a>
                                    </div>
                                }
                            }
                        >

                            <div class="flex items-center">
                                <NextGameButton time_mode=store_value(TimeMode::RealTime)/>
                                <NextGameButton time_mode=store_value(TimeMode::Correspondence)/>
                                <NextGameButton time_mode=store_value(TimeMode::Untimed)/>
                            </div>
                            <div class="flex items-center lg:mr-10">
                                <ChatAndControls/>
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
