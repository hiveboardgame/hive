use crate::{components::molecules::hamburger::Hamburger, i18n::*};
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn MobileDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let div_style = "ui-mobile-dropdown-body font-bold dark:text-white";
    let label_style = "ui-mobile-dropdown-label";
    let section_style = "ui-mobile-dropdown-section";
    let i18n = use_i18n();
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="ui-header-icon-button lg:hidden"
            extend_tw_classes="h-full"
            dropdown_style="ui-dropdown-menu ui-dropdown-menu-left ui-header-dropdown-menu ui-mobile-dropdown-menu"
            content=view! { <Icon icon=icondata_ch::ChMenuHamburger attr:class="size-6" /> }
            id="Mobile"
            aria_label="Open navigation menu"
        >

            <div class=div_style>
                <div class=section_style>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/">
                        {t!(i18n, header.home)}
                    </a>
                </div>
                <div class=section_style>
                    <span class=label_style>{t!(i18n, header.community.title)}</span>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/top_players">
                        {t!(i18n, header.community.top_players)}
                    </a>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/resources">
                        {t!(i18n, header.community.resources)}
                    </a>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/faq">
                        {t!(i18n, header.community.faq)}
                    </a>
                </div>
                <div class=section_style>
                    <span class=label_style>{t!(i18n, header.learn.title)}</span>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/analysis">
                        {t!(i18n, header.learn.analysis)}
                    </a>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/archive">
                        {t!(i18n, header.learn.archive)}
                    </a>
                    <a
                        class="ui-dropdown-link"
                        on:click=onclick_close
                        href="https://hivegame.com/download/rules.pdf"
                        target="_blank"
                    >
                        {t!(i18n, header.learn.rules)}
                    </a>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/rules_summary">
                        {t!(i18n, header.learn.rules_summary)}
                    </a>
                </div>
                <div class=section_style>
                    <span class=label_style>{t!(i18n, header.tournaments.title)}</span>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/tournaments">
                        {t!(i18n, header.tournaments.view)}
                    </a>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/tournaments/create">
                        {t!(i18n, header.tournaments.create)}
                    </a>
                </div>
                <div class=section_style>
                    <span class=label_style>{t!(i18n, header.support)}</span>
                    <a
                        class="ui-dropdown-link"
                        on:click=onclick_close
                        href="https://www.gen42.com/"
                        target="_blank"
                        rel="external"
                    >
                        {t!(i18n, header.buy_game)}
                    </a>
                    <a class="ui-dropdown-link" on:click=onclick_close href="/donate">
                        {t!(i18n, header.donate)}
                    </a>
                </div>
            </div>

        </Hamburger>
    }
}
