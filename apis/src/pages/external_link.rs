use crate::{
    common::external_link_destination,
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::page_card::PageCard,
    },
    i18n::{t, use_i18n},
};
use leptos::{either::EitherOf3, prelude::*};
use leptos_router::hooks::use_location;
use url::Url;

#[component]
pub fn ExternalLink() -> impl IntoView {
    let i18n = use_i18n();
    let location = use_location();
    let destination = RwSignal::new(None::<Option<Url>>);
    // The server cannot see fragments; read them after hydration to keep SSR consistent.
    Effect::new(move |_| {
        destination.set(Some(external_link_destination(&location.hash.get())));
    });

    view! {
        <PageShell variant=PageShellVariant::Form>
            <PageCard class="p-6 space-y-6 sm:p-8">
                <h1 class="ui-page-title">{t!(i18n, messages.external_link.title)}</h1>
                {move || match destination.get() {
                    None => {
                        EitherOf3::A(
                            view! {
                                <p class="ui-page-subtitle" role="status">
                                    {t!(i18n, messages.external_link.loading)}
                                </p>
                            },
                        )
                    }
                    Some(None) => {
                        EitherOf3::B(
                            view! {
                                <p class="ui-field-error" role="alert">
                                    {t!(i18n, messages.external_link.invalid_destination)}
                                </p>
                            },
                        )
                    }
                    Some(Some(url)) => {
                        EitherOf3::C(
                            view! {
                                <div class="space-y-4">
                                    <p class="ui-page-subtitle">
                                        {t!(i18n, messages.external_link.confirmation)}
                                    </p>
                                    <div class="p-4 space-y-2 rounded-lg border select-text border-pillbug-teal/30">
                                        <p
                                            class="text-lg font-semibold [overflow-wrap:anywhere]"
                                            dir="ltr"
                                        >
                                            {url.host_str().unwrap_or_default().to_string()}
                                        </p>
                                        <p
                                            class="text-sm ui-page-subtitle [overflow-wrap:anywhere]"
                                            dir="ltr"
                                        >
                                            {url.to_string()}
                                        </p>
                                    </div>
                                    <p class="ui-page-subtitle">
                                        {t!(i18n, messages.external_link.safety_reminder)}
                                    </p>
                                </div>
                            },
                        )
                    }
                }}
                <div class="flex flex-col gap-3 sm:flex-row sm:justify-end">
                    <a class="ui-button ui-button-secondary ui-button-md" href="/">
                        {t!(i18n, messages.external_link.back_to_hivegame)}
                    </a>
                    {move || {
                        destination
                            .get()
                            .flatten()
                            .map(|url| {
                                view! {
                                    <a
                                        class="ui-button ui-button-primary ui-button-md"
                                        href=url.to_string()
                                        rel="external noopener noreferrer"
                                        target="_self"
                                    >
                                        {t!(i18n, messages.external_link.continue_to_site)}
                                    </a>
                                }
                            })
                    }}
                </div>
            </PageCard>
        </PageShell>
    }
}
