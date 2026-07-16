use crate::{
    common::with_class,
    components::{
        atoms::next_game_button::NextGameButton,
        molecules::chat_and_controls::ChatAndControls,
        organisms::{
            darkmode_toggle::{DarkModeToggle, DarkModeToggleVariant},
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
use leptos_router::hooks::{use_location, use_params_map};
use shared_types::{GameId, TimeMode};

const HEADER_NAV_ITEM_CLASS: &str =
    "flex h-full items-center px-2 py-0 font-bold whitespace-nowrap transition-colors duration-200 active:scale-100 no-link-style hover:bg-black/5 hover:text-pillbug-teal dark:hover:bg-white/10";

#[component]
pub fn Header() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let i18n = use_i18n();
    let location = use_location();
    let params = use_params_map();
    let current_game_id = Signal::derive(move || {
        let nanoid = params.get().get("nanoid")?.to_string();
        (location.pathname.get() == format!("/game/{nanoid}")).then_some(GameId(nanoid))
    });
    view! {
        <header class=with_class(
            "ui-top-bar-surface",
            "fixed top-0 z-50 flex h-10 w-full max-w-[100vw] items-center justify-between select-none",
        )>
            <div class="flex relative z-10 gap-0 items-center h-full">
                <MobileDropdown />
                <div class="hidden h-full lg:flex lg:gap-0 lg:items-center">
                    <a class=HEADER_NAV_ITEM_CLASS href="/">
                        {t!(i18n, header.home)}
                    </a>
                    <CommunityDropdown />
                    <LearnDropdown />
                    <TournamentDropdown />
                    <a
                        class=HEADER_NAV_ITEM_CLASS
                        href="https://www.gen42.com/"
                        rel="external"
                        target="_blank"
                    >
                        {t!(i18n, header.buy_game)}
                    </a>
                    <a
                        class="flex items-center py-0 px-2 h-full font-bold uppercase whitespace-nowrap transition-colors duration-200 active:scale-100 no-link-style text-[#2A6560] dark:text-[#FAB93F] dark:hover:bg-white/10 hover:bg-black/5 hover:text-pillbug-teal"
                        href="/donate"
                    >
                        {t!(i18n, header.donate)}
                    </a>
                </div>
            </div>
            <Controls user=auth_context.user current_game_id />
        </header>
    }
}

#[component]
fn GuestActions(current_game_id: Signal<Option<GameId>>) -> impl IntoView {
    let referrer = expect_context::<RefererContext>().pathname;
    view! {
        <div class="flex items-center mr-1 h-full">
            <ChatAndControls current_game_id />
            <SoundToggle />
            <LocaleDropdown />
            <DarkModeToggle variant=DarkModeToggleVariant::Header />
            <a
                class="ui-header-login-button no-link-style"
                href="/login"
                on:focus=move |_| set_redirect(referrer)
            >
                Login
            </a>
        </div>
    }
}

#[component]
fn Controls(
    user: Signal<Option<AccountResponse>>,
    current_game_id: Signal<Option<GameId>>,
) -> impl IntoView {
    move || match user() {
        Some(user) => {
            let games = expect_context::<GamesSignal>();
            Either::Left(view! {
                <div class="flex items-center h-full xl:absolute xl:top-0 xl:left-1/2 xl:-translate-x-1/2">
                    <NextGameButton time_mode=TimeMode::RealTime games />
                    <NextGameButton time_mode=TimeMode::Correspondence games />
                    <NextGameButton time_mode=TimeMode::Untimed games />
                </div>
                <div class="flex relative z-10 items-center h-full">
                    <ChatAndControls current_game_id />
                    <SoundToggle />
                    <LocaleDropdown />
                    <NotificationDropdown current_game_id />
                    <UserDropdown username=user.username.clone() current_game_id />
                </div>
            })
        }
        None => Either::Right(view! { <GuestActions current_game_id /> }),
    }
}

pub fn set_redirect(referrer: StoredValue<String>) {
    referrer.set_value(use_location().pathname.get_untracked());
}
