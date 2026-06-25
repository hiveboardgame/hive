use crate::{
    common::render_text_prop,
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::panel::Panel,
    },
    i18n::*,
};
use leptos::prelude::*;

#[component]
fn FaqSection(children: Children, #[prop(into)] title: TextProp) -> impl IntoView {
    view! {
        <Panel title=title body_class="space-y-3">
            {children()}
        </Panel>
    }
}

#[component]
fn FaqItem(children: Children, #[prop(into)] question: TextProp) -> impl IntoView {
    view! {
        <details class="group">
            <summary class="justify-between ui-disclosure-summary">
                <span>{render_text_prop(question)}</span>
                <span class="flex-shrink-0 ml-1.5 transition-transform group-open:rotate-180">
                    <svg class="size-5" viewBox="0 0 20 20" fill="currentColor">
                        <path
                            fill-rule="evenodd"
                            d="M5.293 7.293a1 1 0 011.414 0L10 10.586l3.293-3.293a1 1 0 111.414 1.414l-4 4a1 1 0 01-1.414 0l-4-4a1 1 0 010-1.414z"
                            clip-rule="evenodd"
                        />
                    </svg>
                </span>
            </summary>
            <div class="px-3 pt-0 pb-3 text-sm leading-6 text-gray-600 dark:text-gray-300">
                {children()}
            </div>
        </details>
    }
}

#[component]
pub fn Faq() -> impl IntoView {
    let i18n = use_i18n();
    let rel = "external";
    let target = "_blank";
    let link_class = "ui-text-link";
    view! {
        <PageShell>
            <PageHeader title=move || { t_string!(i18n, faq.title) } />
            <div class="space-y-6">

                <FaqSection title=move || { t_string!(i18n, faq.sections.getting_started) }>
                    <FaqItem question=move || { t_string!(i18n, faq.what_is_hive.question) }>
                        <p>
                            {t!(
                                i18n, faq.what_is_hive.answer,
                                            < gen42_link > =
                                            <a href="https://www.gen42.com/" rel=rel target=target class=link_class/>
                            )}
                        </p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.hive_rules.question) }>
                        <p>
                            {t!(
                                i18n, faq.hive_rules.answer,
                                            < rules_link > =
                                            <a href="https://hivegame.com/download/rules.pdf" target=target class=link_class/>,
                                            < video_link > =
                                            <a href="https://www.youtube.com/watch?v=-_CT8cgOR5Q" rel=rel target=target class=link_class/>
                            )}
                        </p>

                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.expansions_supported.question)
                    }>
                        <p>{t!(i18n, faq.expansions_supported.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.can_i_play_base_game.question)
                    }>
                        <p>{t!(i18n, faq.can_i_play_base_game.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.resign_ok.question) }>
                        <p>{t!(i18n, faq.resign_ok.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.signup_required.question) }>
                        <p>{t!(i18n, faq.signup_required.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.mobile_friendly.question) }>
                        <p>{t!(i18n, faq.mobile_friendly.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.mobile_app.question) }>
                        <p>{t!(i18n, faq.mobile_app.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.profile_navigation.question) }>
                        <p>{t!(i18n, faq.profile_navigation.answer)}</p>

                    </FaqItem>
                </FaqSection>

                <FaqSection title=move || { t_string!(i18n, faq.sections.game_settings) }>
                    <FaqItem question=move || { t_string!(i18n, faq.game_types.question) }>
                        <p>{t!(i18n, faq.game_types.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.start_game.question) }>
                        <p>{t!(i18n, faq.start_game.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.time_controls.question) }>
                        <p>{t!(i18n, faq.time_controls.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.realtime_vs_correspondence.question)
                    }>
                        <p>{t!(i18n, faq.realtime_vs_correspondence.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.casual_vs_rated.question) }>
                        <p>{t!(i18n, faq.casual_vs_rated.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.why_elo_has_questionmark.question)
                    }>
                        <p>{t!(i18n, faq.why_elo_has_questionmark.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.base_vs_mlp.question) }>
                        <p>{t!(i18n, faq.base_vs_mlp.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.private_vs_public.question) }>
                        <p>{t!(i18n, faq.private_vs_public.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.rating_range.question) }>
                        <p>{t!(i18n, faq.rating_range.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.color_selection.question) }>
                        <p>{t!(i18n, faq.color_selection.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.how_to_check_stack.question) }>
                        <p>{t!(i18n, faq.how_to_check_stack.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.spectate_games.question) }>
                        <p>{t!(i18n, faq.spectate_games.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.turn_alerts.question) }>
                        <p>{t!(i18n, faq.turn_alerts.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.correspondence_notifications.question)
                    }>
                        <p>{t!(i18n, faq.correspondence_notifications.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.chat_basics.question) }>
                        <p>{t!(i18n, faq.chat_basics.answer)}</p>

                    </FaqItem>
                </FaqSection>

                <FaqSection title=move || { t_string!(i18n, faq.sections.tournaments) }>
                    <FaqItem question=move || {
                        t_string!(i18n, faq.can_i_set_up_a_tournament.question)
                    }>
                        <p>{t!(i18n, faq.can_i_set_up_a_tournament.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_entry.question) }>
                        <p>{t!(i18n, faq.tournament_entry.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_duration.question) }>
                        <p>{t!(i18n, faq.tournament_duration.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_stats.question) }>
                        <p>{t!(i18n, faq.tournament_stats.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_games.question) }>
                        <p>{t!(i18n, faq.tournament_games.answer)}</p>

                    </FaqItem>
                </FaqSection>

                <FaqSection title=move || { t_string!(i18n, faq.sections.account_configuration) }>
                    <FaqItem question=move || { t_string!(i18n, faq.change_password.question) }>
                        <p>
                            {t!(
                                i18n, faq.change_password.answer,
                                            < edit_link > =
                                            <a href="/account/edit" rel=rel target=target class=link_class/>
                            )}
                        </p>

                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.how_to_recover_password.question)
                    }>
                        <p>{t!(i18n, faq.how_to_recover_password.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.move_confirmation.question) }>
                        <p>{t!(i18n, faq.move_confirmation.answer)}</p>

                    </FaqItem>
                </FaqSection>

                <FaqSection title=move || { t_string!(i18n, faq.sections.more_features) }>
                    <FaqItem question=move || { t_string!(i18n, faq.game_analysis.question) }>
                        <p>{t!(i18n, faq.game_analysis.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.play_bots.question) }>
                        <p>{t!(i18n, faq.play_bots.answer)}</p>

                    </FaqItem>
                </FaqSection>

                <FaqSection title=move || { t_string!(i18n, faq.sections.community_support) }>
                    <FaqItem question=move || { t_string!(i18n, faq.get_better.question) }>
                        <p>
                            {t!(
                                i18n, faq.get_better.answer,
                                            < resources_link > =
                                            <a href="/resources" rel=rel target=target class=link_class/>
                            )}
                        </p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.community_forum.question) }>
                        <p>
                            {t!(
                                i18n, faq.community_forum.answer,
                                            < discord_link > =
                                            <a href="https://discord.gg/djdQZPFa7E" rel=rel target=target class=link_class/>
                            )}
                        </p>

                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.play_specific_players.question)
                    }>
                        <p>{t!(i18n, faq.play_specific_players.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.report_bugs.question) }>
                        <p>
                            {t!(
                                i18n, faq.report_bugs.answer,
                                            < discord_link > =
                                            <a href="https://discord.gg/jNTjr5vj9Z" rel=rel target=target class=link_class/>
                            )}
                        </p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.how_to_help.question) }>
                        <div>
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

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.opponent_is_abusive.question) }>
                        <p>{t!(i18n, faq.opponent_is_abusive.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.why_not_in_top_players.question)
                    }>
                        <p>{t!(i18n, faq.why_not_in_top_players.answer)}</p>

                    </FaqItem>
                </FaqSection>

                <FaqSection title=move || { t_string!(i18n, faq.sections.donating) }>
                    <FaqItem question=move || { t_string!(i18n, faq.how_to_get_crown.question) }>
                        <p>{t!(i18n, faq.how_to_get_crown.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.donation_amount.question) }>
                        <p>{t!(i18n, faq.donation_amount.answer)}</p>

                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.how_donate.question) }>
                        <p>
                            {t!(
                                i18n, faq.how_donate.answer,
                                            < donate_link > =
                                            <a href="/donate" rel=rel target=target class=link_class/>
                            )}
                        </p>

                    </FaqItem>
                </FaqSection>
            </div>
        </PageShell>
    }
}
