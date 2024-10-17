use crate::components::atoms::next_game_button::NextGameButton;
use crate::components::molecules::chat_and_controls::ChatAndControls;
use crate::components::organisms::{
    darkmode_toggle::DarkModeToggle,
    dropdowns::{
        community::CommunityDropdown, learn::LearnDropdown, locale::LocaleDropdown,
        mobile::MobileDropdown, notification::NotificationDropdown, tournament::TournamentDropdown,
        user::UserDropdown,
    },
    sound_toggle::SoundToggle,
};
use crate::i18n::*;
use crate::providers::AuthContext;
use leptos::*;
use leptos_router::use_location;
use shared_types::TimeMode;

#[derive(Clone)]
pub struct Redirect(pub RwSignal<String>);

#[component]
pub fn Header() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let username = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user.username),
        _ => None,
    };
    let i18n = use_i18n();
    view! {
        <header class="w-full fixed top-0 flex justify-between items-center bg-gray-300 dark:bg-header-twilight z-50 max-w-[100vw] select-none">
            <div class="flex gap-1 items-center">
                <MobileDropdown/>
                <div class="hidden lg:flex lg:items-center lg:gap-1">
                    <a
                        class="block p-2 h-full font-bold whitespace-nowrap transition-transform duration-300 transform hover:text-pillbug-teal active:scale-95"
                        href="/"
                    >
                        {t!(i18n, header.home)}
                    </a>
                    <CommunityDropdown/>
                    <LearnDropdown/>
                    <TournamentDropdown/>
                    <a
                        class="block p-2 h-full font-bold whitespace-nowrap transition-transform duration-300 transform hover:text-pillbug-teal active:scale-95"
                        href="https://www.gen42.com/"
                        rel="external"
                        target="_blank"
                    >
                        {t!(i18n, header.buy_game)}
                    </a>
                    <a
                        class="block p-2 h-full font-bold uppercase whitespace-nowrap transition-transform duration-300 transform dark:text-[#FAB93F] text-[#2A6560] hover:text-pillbug-teal active:scale-95"
                        href="/donate"
                    >
                        {t!(i18n, header.donate)}
                    </a>
                </div>
            </div>
            <Transition fallback=|| view! { <GuestActions/> }>
                <Show when=move || username().is_some() fallback=|| view! { <GuestActions/> }>
                    <div class="flex items-center">
                        <NextGameButton time_mode=store_value(TimeMode::RealTime)/>
                        <NextGameButton time_mode=store_value(TimeMode::Correspondence)/>
                        <NextGameButton time_mode=store_value(TimeMode::Untimed)/>
                    </div>
                    <div class="flex items-center mr-1">
                        <ChatAndControls/>
                        <SoundToggle/>
                        <LocaleDropdown/>
                        <NotificationDropdown/>
                        <UserDropdown username=username().expect("Username is some")/>
                    </div>
                </Show>
            </Transition>
        </header>
    }
}

#[component]
fn GuestActions() -> impl IntoView {
    view! {
        <div class="flex items-center mr-1">
            <ChatAndControls/>
            <SoundToggle/>
            <LocaleDropdown/>
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

pub fn set_redirect() {
    let referrer = RwSignal::new(String::from("/"));
    let location = use_location().pathname.get();
    referrer.update(|s| *s = location);
    provide_context(Redirect(referrer));
}
