use leptos::prelude::*;

use crate::components::{layouts::base_layout::COMMON_LINK_STYLE, molecules::banner::Banner};
use crate::i18n::*;

#[component]
pub fn Donate() -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="pt-20">
            <div class="px-4 mx-auto max-w-4xl sm:px-6 lg:px-8">
                <Banner
                    title=t!(i18n, donate.title).into_any()
                    text=t_string!(i18n, donate.subtitle).into()
                />
                <p class="my-4 text-lg text-center">{t!(i18n, donate.about)}</p>
                <div class="flex justify-center items-center my-4">
                    {t!(i18n, donate.kofi_button, < kofi_button > = <a href="https://ko-fi.com/hivedevs" class=COMMON_LINK_STYLE/>)}
                    {t!(i18n, donate.patreon_button, < patreon_button > = <a href="https://www.patreon.com/HiveDevs" class=COMMON_LINK_STYLE/>)}
                </div>

                <div class="p-3">
                    <h3 class="text-lg font-medium leading-6">
                        {t!(i18n, donate.where_does_money_go.question)}
                    </h3>
                    <p class="mt-2 text-base">{t!(i18n, donate.where_does_money_go.answer)}</p>
                </div>

                <div class="p-3">
                    <h3 class="text-lg font-medium leading-6">
                        {t!(i18n, donate.features_for_patrons.question)}
                    </h3>
                    <p class="mt-2 text-base">{t!(i18n, donate.features_for_patrons.answer)}</p>
                </div>

                <div class="mt-4 text-center">{t!(i18n, donate.small_team_blurb)}</div>
            </div>
        </div>
    }
}
