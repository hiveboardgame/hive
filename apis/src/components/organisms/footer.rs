use crate::{
    common::{
        render_text_prop,
        CONTRIBUTING_URL,
        GEN42_URL,
        GITHUB_ISSUES_URL,
        GITHUB_URL,
        HIVEGAME_DISCORD_URL,
        KOFI_URL,
        LICENSE_URL,
        PATREON_URL,
        WORLD_HIVE_DISCORD_URL,
    },
    i18n::*,
    providers::AuthContext,
};
use leptos::prelude::*;
use leptos_icons::Icon;

const FOOTER_LINK_CLASS: &str = "no-link-style text-gray-600 transition-colors hover:text-pillbug-teal dark:text-gray-300 dark:hover:text-[#FAB93F]";

#[component]
pub fn Footer() -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let is_donor = move || {
        auth_context
            .user
            .with(|user| user.as_ref().is_some_and(|account| account.user.patreon))
    };

    view! {
        <footer class="mt-auto w-full text-sm border-t border-black/10 bg-odd-light/80 dark:border-white/10 dark:bg-surface-muted">
            <div class="flex flex-col gap-8 py-8 px-4 mx-auto max-w-5xl sm:px-6">
                <Show when=is_donor fallback=|| view! { <SupportAppeal /> }>
                    <p class="font-semibold text-gray-900 dark:text-gray-100">
                        {t!(i18n, footer.thanks)}
                    </p>
                </Show>
                <nav class="grid grid-cols-2 gap-6 sm:grid-cols-3">
                    <FooterColumn title=move || t_string!(i18n, footer.community.title)>
                        <FooterLink href=HIVEGAME_DISCORD_URL>
                            {t!(i18n, footer.community.hivegame_discord)}
                        </FooterLink>
                        <FooterLink href=WORLD_HIVE_DISCORD_URL>
                            {t!(i18n, footer.community.world_hive_discord)}
                        </FooterLink>
                        <FooterLink href="/resources">
                            {t!(i18n, footer.community.resources)}
                        </FooterLink>
                        <FooterLink href="/faq">{t!(i18n, footer.community.faq)}</FooterLink>
                    </FooterColumn>
                    <FooterColumn title=move || t_string!(i18n, footer.open_source.title)>
                        <FooterLink href=GITHUB_URL>
                            {t!(i18n, footer.open_source.source_code)}
                        </FooterLink>
                        <FooterLink href=CONTRIBUTING_URL>
                            {t!(i18n, footer.open_source.contribute)}
                        </FooterLink>
                        <FooterLink href=GITHUB_ISSUES_URL>
                            {t!(i18n, footer.open_source.report_bug)}
                        </FooterLink>
                        <FooterLink href=LICENSE_URL>
                            {t!(i18n, footer.open_source.license)}
                        </FooterLink>
                    </FooterColumn>
                    <FooterColumn title=move || t_string!(i18n, footer.support.title)>
                        <FooterLink href="/donate">{t!(i18n, footer.support.donate)}</FooterLink>
                        <FooterLink href=KOFI_URL>{t!(i18n, footer.support.kofi)}</FooterLink>
                        <FooterLink href=PATREON_URL>{t!(i18n, footer.support.patreon)}</FooterLink>
                        <FooterLink href=GEN42_URL>{t!(i18n, footer.support.buy_game)}</FooterLink>
                    </FooterColumn>
                </nav>
                <div class="flex flex-col gap-3 pt-6 border-t sm:flex-row sm:gap-6 sm:justify-between sm:items-center border-black/10 dark:border-white/10">
                    <div class="flex flex-wrap gap-y-2 gap-x-4 items-center">
                        <FooterLink href=GITHUB_URL label="GitHub">
                            <Icon icon=icondata_fa::FaGithubBrands attr:class="size-5" />
                        </FooterLink>
                        <FooterLink href=HIVEGAME_DISCORD_URL label="Discord">
                            <Icon icon=icondata_fa::FaDiscordBrands attr:class="size-5" />
                        </FooterLink>
                        <FooterLink href="/privacy">{t!(i18n, footer.legal.privacy)}</FooterLink>
                        <FooterLink href="/terms">{t!(i18n, footer.legal.terms)}</FooterLink>
                    </div>
                    <p class="text-xs text-gray-500 sm:max-w-md sm:text-right dark:text-gray-400">
                        {t!(i18n, footer.legal.trademark)}
                    </p>
                </div>
            </div>
        </footer>
    }
}

#[component]
fn SupportAppeal() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <section class="flex flex-col gap-3 sm:flex-row sm:gap-6 sm:justify-between sm:items-center">
            <div class="flex flex-col gap-1">
                <h2 class="text-base font-bold text-gray-900 dark:text-gray-100">
                    {t!(i18n, footer.appeal.title)}
                </h2>
                <p class="text-gray-600 dark:text-gray-300">{t!(i18n, footer.appeal.body)}</p>
            </div>
            <a
                href="/donate"
                class="self-start sm:self-auto ui-button ui-button-primary ui-button-md no-link-style shrink-0"
            >
                <Icon icon=icondata_fa::FaHeartSolid attr:class="size-4" />
                {t!(i18n, footer.appeal.button)}
            </a>
        </section>
    }
}

#[component]
fn FooterColumn(#[prop(into)] title: TextProp, children: Children) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-2">
            <h2 class="text-xs font-bold tracking-wide text-gray-900 uppercase dark:text-gray-100">
                {render_text_prop(title)}
            </h2>
            <div class="flex flex-col gap-1.5 items-start">{children()}</div>
        </div>
    }
}

#[component]
fn FooterLink(
    href: &'static str,
    #[prop(optional)] label: Option<&'static str>,
    children: Children,
) -> impl IntoView {
    let external = href.starts_with("https://");
    view! {
        <a
            href=href
            class=FOOTER_LINK_CLASS
            aria-label=label
            rel=external.then_some("external")
            target=external.then_some("_blank")
        >
            {children()}
        </a>
    }
}
