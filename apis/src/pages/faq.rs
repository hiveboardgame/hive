use crate::components::molecules::banner::Banner;
use crate::i18n::*;
use leptos::prelude::*;

#[component]
pub fn Faq() -> impl IntoView {
    let i18n = use_i18n();
    let rel = "external";
    let target = "_blank";
    let link_class = "text-blue-500 hover:underline";

    view! {
        <div class="pt-20 pb-20">
            <div class="px-4 mx-auto w-full max-w-4xl sm:px-6 lg:px-8">
                <Banner title=t!(i18n, faq.title) />

                <div class="space-y-6">

                    <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                        <h2 class="mb-4 text-xl font-bold text-center text-blue-600 dark:text-blue-400">
                            {t!(i18n, faq.sections.getting_started)}
                        </h2>
                        <div class="space-y-3">
                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.what_is_hive.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(
                                            i18n, faq.what_is_hive.answer,
                                            < gen42_link > =
                                            <a href="https://www.gen42.com/" rel=rel target=target class=link_class/>
                                        )}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.hive_rules.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(
                                            i18n, faq.hive_rules.answer,
                                            < rules_link > =
                                            <a href="https://hivegame.com/download/rules.pdf" target=target class=link_class/>,
                                            < video_link > =
                                            <a href="https://www.youtube.com/watch?v=-_CT8cgOR5Q" rel=rel target=target class=link_class/>
                                        )}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.expansions_supported.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.expansions_supported.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.can_i_play_base_game.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.can_i_play_base_game.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.resign_ok.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.resign_ok.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.signup_required.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.signup_required.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.mobile_friendly.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.mobile_friendly.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.mobile_app.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.mobile_app.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.profile_navigation.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.profile_navigation.answer)}
                                    </p>
                                </div>
                            </details>
                        </div>
                    </div>

                    <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                        <h2 class="mb-4 text-xl font-bold text-center text-green-600 dark:text-green-400">
                            {t!(i18n, faq.sections.game_settings)}
                        </h2>
                        <div class="space-y-3">
                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.game_types.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.game_types.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.start_game.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.start_game.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.time_controls.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.time_controls.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.realtime_vs_correspondence.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.realtime_vs_correspondence.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.casual_vs_rated.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.casual_vs_rated.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.why_elo_has_questionmark.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.why_elo_has_questionmark.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.base_vs_mlp.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.base_vs_mlp.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.private_vs_public.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.private_vs_public.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.rating_range.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.rating_range.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.color_selection.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.color_selection.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.how_to_check_stack.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.how_to_check_stack.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.spectate_games.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.spectate_games.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.turn_alerts.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.turn_alerts.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>
                                        {t!(i18n, faq.correspondence_notifications.question)}
                                    </span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.correspondence_notifications.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.chat_basics.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.chat_basics.answer)}
                                    </p>
                                </div>
                            </details>
                        </div>
                    </div>

                    <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                        <h2 class="mb-4 text-xl font-bold text-center text-purple-600 dark:text-purple-400">
                            {t!(i18n, faq.sections.tournaments)}
                        </h2>
                        <div class="space-y-3">
                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.can_i_set_up_a_tournament.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.can_i_set_up_a_tournament.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.tournament_entry.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.tournament_entry.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.tournament_duration.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.tournament_duration.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.tournament_stats.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.tournament_stats.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.tournament_games.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.tournament_games.answer)}
                                    </p>
                                </div>
                            </details>
                        </div>
                    </div>

                    <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                        <h2 class="mb-4 text-xl font-bold text-center text-orange-600 dark:text-orange-400">
                            {t!(i18n, faq.sections.account_configuration)}
                        </h2>
                        <div class="space-y-3">
                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.change_password.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(
                                            i18n, faq.change_password.answer,
                                            < edit_link > =
                                            <a href="/account/edit" rel=rel target=target class=link_class/>
                                        )}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.how_to_recover_password.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.how_to_recover_password.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.move_confirmation.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.move_confirmation.answer)}
                                    </p>
                                </div>
                            </details>
                        </div>
                    </div>

                    <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                        <h2 class="mb-4 text-xl font-bold text-center text-red-600 dark:text-red-400">
                            {t!(i18n, faq.sections.more_features)}
                        </h2>
                        <div class="space-y-3">
                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.game_analysis.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.game_analysis.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.play_bots.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.play_bots.answer)}
                                    </p>
                                </div>
                            </details>
                        </div>
                    </div>

                    <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                        <h2 class="mb-4 text-xl font-bold text-center text-indigo-600 dark:text-indigo-400">
                            {t!(i18n, faq.sections.community_support)}
                        </h2>
                        <div class="space-y-3">
                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.get_better.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(
                                            i18n, faq.get_better.answer,
                                            < resources_link > =
                                            <a href="/resources" rel=rel target=target class=link_class/>
                                        )}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.community_forum.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(
                                            i18n, faq.community_forum.answer,
                                            < discord_link > =
                                            <a href="https://discord.gg/djdQZPFa7E" rel=rel target=target class=link_class/>
                                        )}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.play_specific_players.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.play_specific_players.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.report_bugs.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(
                                            i18n, faq.report_bugs.answer,
                                            < discord_link > =
                                            <a href="https://discord.gg/jNTjr5vj9Z" rel=rel target=target class=link_class/>
                                        )}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.how_to_help.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <div class="text-base text-gray-600 dark:text-gray-400">
                                        <ul class="mt-2 list-disc list-inside">
                                            <li>{t!(i18n, faq.how_to_help.answers.item1)}</li>
                                            <li>
                                                {t!(
                                                    i18n, faq.how_to_help.answers.item2,
                                                    < github_link > =
                                                    <a href="https://github.com/hiveboardgame/hive" rel=rel target=target class=link_class/>
                                                )}
                                            </li>
                                            <li>{t!(i18n, faq.how_to_help.answers.item3)}</li>
                                        </ul>
                                    </div>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.opponent_is_abusive.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.opponent_is_abusive.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.why_not_in_top_players.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.why_not_in_top_players.answer)}
                                    </p>
                                </div>
                            </details>
                        </div>
                    </div>

                    <div class="px-8 pt-6 pb-8 mb-6 rounded-lg border shadow-lg bg-stone-300 dark:bg-slate-800 border-stone-400 dark:border-slate-600">
                        <h2 class="mb-4 text-xl font-bold text-center text-yellow-600 dark:text-yellow-400">
                            {t!(i18n, faq.sections.donating)}
                        </h2>
                        <div class="space-y-3">
                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.how_to_get_crown.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.how_to_get_crown.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.donation_amount.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(i18n, faq.donation_amount.answer)}
                                    </p>
                                </div>
                            </details>

                            <details class="group">
                                <summary class="flex justify-between items-center p-3 font-medium text-gray-700 rounded-lg cursor-pointer dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700">
                                    <span>{t!(i18n, faq.how_donate.question)}</span>
                                    <span class="flex-shrink-0 ml-1.5 transition group-open:rotate-180">
                                        <svg
                                            class="w-5 h-5"
                                            viewBox="0 0 20 20"
                                            fill="currentColor"
                                        >
                                            <path
                                                fill-rule="evenodd"
                                                d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                                                clip-rule="evenodd"
                                            />
                                        </svg>
                                    </span>
                                </summary>
                                <div class="px-3 pt-0 pb-3">
                                    <p class="text-base text-gray-600 dark:text-gray-400">
                                        {t!(
                                            i18n, faq.how_donate.answer,
                                            < donate_link > =
                                            <a href="/donate" rel=rel target=target class=link_class/>
                                        )}
                                    </p>
                                </div>
                            </details>
                        </div>
                    </div>

                </div>
            </div>
        </div>
    }
}
