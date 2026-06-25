use leptos::prelude::*;

use crate::{
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::panel::Panel,
    },
    i18n::*,
};

#[component]
pub fn Donate() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <PageShell>
            <PageHeader
                title=move || { t_string!(i18n, donate.title) }
                subtitle=move || { t_string!(i18n, donate.subtitle) }
            />
            <p class="ui-notice">{t!(i18n, donate.about)}</p>
            <div class="flex flex-wrap gap-3 justify-center">
                {t!(
                    i18n,
                        donate.kofi_button,
                        < kofi_button > =
                        <a href="https://ko-fi.com/hivedevs" class="ui-button ui-button-primary ui-button-md no-link-style"/>
                )}
                {t!(
                    i18n,
                        donate.patreon_button,
                        < patreon_button > =
                        <a href="https://www.patreon.com/HiveDevs" class="ui-button ui-button-secondary ui-button-md no-link-style"/>
                )}
            </div>

            <div class="grid gap-4 md:grid-cols-2">
                <Panel title=move || { t_string!(i18n, donate.where_does_money_go.question) }>
                    <p class="text-sm leading-6 text-gray-700 dark:text-gray-300">
                        {t!(i18n, donate.where_does_money_go.answer)}
                    </p>
                </Panel>

                <Panel title=move || { t_string!(i18n, donate.features_for_patrons.question) }>
                    <p class="text-sm leading-6 text-gray-700 dark:text-gray-300">
                        {t!(i18n, donate.features_for_patrons.answer)}
                    </p>
                </Panel>
            </div>

            <p class="text-sm text-center text-gray-600 dark:text-gray-300">
                {t!(i18n, donate.small_team_blurb)}
            </p>
        </PageShell>
    }
}
