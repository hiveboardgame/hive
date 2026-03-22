use crate::{
    components::{
        atoms::next_game_button::NextGameButton,
        molecules::chat_and_controls::ChatAndControls,
        organisms::{
            darkmode_toggle::DarkModeToggle,
            dropdowns::{
                community::CommunityDropdown,
                learn::LearnDropdown,
                locale::LocaleDropdown,
                mobile::MobileDropdown,
                notification::NotificationDropdown,
                tournament::TournamentDropdown,
                user::UserDropdown,
            },
            sound_toggle::SoundToggle,
        },
    },
    i18n::*,
    providers::{games::GamesSignal, AuthContext, RefererContext},
    responses::AccountResponse,
};
use leptos::{either::Either, prelude::*};
use leptos_router::hooks::use_location;
use shared_types::TimeMode;

#[component]
pub fn Header() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let i18n = use_i18n();
    view! {
        <header class="flex fixed top-0 z-50 justify-between items-center w-full h-10 bg-gray-300 select-none max-w-[100vw] dark:bg-header-twilight">
            <div class="flex gap-1 items-center">
                <MobileDropdown />
                <div class="hidden lg:flex lg:gap-1 lg:items-center">
                    <a
                        class="block p-2 h-full font-bold whitespace-nowrap transition-transform duration-300 active:scale-95 no-link-style hover:text-pillbug-teal"
                        href="/"
                    >
                        {t!(i18n, header.home)}
                    </a>
                    <CommunityDropdown />
                    <LearnDropdown />
                    <TournamentDropdown />
                    <a
                        class="block p-2 h-full font-bold whitespace-nowrap transition-transform duration-300 active:scale-95 no-link-style hover:text-pillbug-teal"
                        href="https://www.gen42.com/"
                        rel="external"
                        target="_blank"
                    >
                        {t!(i18n, header.buy_game)}
                    </a>
                    <a
                        class="block p-2 h-full font-bold uppercase whitespace-nowrap transition-transform duration-300 active:scale-95 no-link-style text-[#2A6560] dark:text-[#FAB93F] hover:text-pillbug-teal"
                        href="/donate"
                    >
                        {t!(i18n, header.donate)}
                    </a>
                </div>
            </div>
            <Controls user=auth_context.user />
        </header>
    }
}

#[component]
fn GuestActions() -> impl IntoView {
    let referrer = expect_context::<RefererContext>().pathname;
    view! {
        <div class="flex items-center mr-1">
            <ChatAndControls />
            <SoundToggle />
            <LocaleDropdown />
            <DarkModeToggle extend_tw_classes="max-h-6 sm:max-h-7" />
            <a
                class="py-1 px-4 m-1 font-bold text-white rounded transition-transform duration-300 active:scale-95 no-link-style bg-button-dawn dark:bg-button-twilight dark:hover:bg-pillbug-teal hover:bg-pillbug-teal"
                href="/login"
                on:focus=move |_| set_redirect(referrer)
            >
                Login
            </a>
        </div>
    }
}

#[component]
fn Controls(user: Signal<Option<AccountResponse>>) -> impl IntoView {
    move || match user() {
        Some(user) => {
            let games = expect_context::<GamesSignal>();
            Either::Left(view! {
                <div class="flex items-center">
                    <NextGameButton time_mode=TimeMode::RealTime games />
                    <NextGameButton time_mode=TimeMode::Correspondence games />
                    <NextGameButton time_mode=TimeMode::Untimed games />
                </div>
                <div class="flex items-center mr-1">
                    <ChatAndControls />
                    <SoundToggle />
                    <LocaleDropdown />
                    <NotificationDropdown />
                    <UserDropdown username=user.username.clone() />
                </div>
            })
        }
        None => Either::Right(view! { <GuestActions /> }),
    }
}

pub fn set_redirect(referrer: StoredValue<String>) {
    referrer.set_value(use_location().pathname.get_untracked());
}
