use crate::{
    common::{
        render_text_prop,
        StandingsCriterionChoice,
        GEN42_URL,
        GITHUB_ISSUES_URL,
        GITHUB_URL,
        HIVEGAME_DISCORD_URL,
        WORLD_HIVE_DISCORD_URL,
    },
    components::{
        layouts::{page_header::PageHeader, page_shell::PageShell},
        molecules::panel::Panel,
    },
    i18n::*,
};
use leptos::{html, leptos_dom::helpers::request_animation_frame_with_handle, prelude::*};
use leptos_icons::Icon;
use shared_types::tournament::{
    round_robin::Criterion as RoundRobin,
    swiss::Criterion as Swiss,
    TournamentPairingNumberOrder,
};
use web_sys::Element;

#[component]
fn FaqSection(
    children: Children,
    #[prop(into)] title: TextProp,
    icon: icondata_core::Icon,
) -> impl IntoView {
    view! {
        <Panel title=title icon=icon body_class="flex flex-col gap-3">
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
                    <svg class="size-5" aria-hidden="true" viewBox="0 0 20 20" fill="currentColor">
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

fn glossary_term(name: impl IntoView, explanation: impl IntoView) -> impl IntoView {
    view! {
        <div>
            <dt class="font-semibold">{name}</dt>
            <dd>{explanation}</dd>
        </div>
    }
}

fn criterion_term(criterion: impl StandingsCriterionChoice) -> impl IntoView {
    let i18n = use_i18n();
    glossary_term(
        move || criterion.name(i18n),
        move || criterion.explanation(i18n),
    )
}

fn filter_faq(root: &Element, query: &str) -> usize {
    let words: Vec<_> = query.split_whitespace().collect();
    let mut total = 0;
    let sections = root.children();
    for section in (0..sections.length()).filter_map(|i| sections.item(i)) {
        let mut matches = 0;
        let items = section.get_elements_by_tag_name("details");
        for item in (0..items.length()).filter_map(|i| items.item(i)) {
            let visible = words.is_empty() || {
                let text = item.text_content().unwrap_or_default().to_lowercase();
                words.iter().all(|word| text.contains(word))
            };
            let _ = item.toggle_attribute_with_force("hidden", !visible);
            matches += usize::from(visible);
        }
        let _ = section.toggle_attribute_with_force("hidden", matches == 0);
        total += matches;
    }
    total
}

#[component]
pub fn Faq() -> impl IntoView {
    let i18n = use_i18n();
    let query = RwSignal::new(String::new());
    let results = RwSignal::new(None::<usize>);
    let faq_ref = NodeRef::<html::Div>::new();
    let input_ref = NodeRef::<html::Input>::new();

    Effect::new(move |_| {
        let query = query.get().trim().to_lowercase();
        let _ = i18n.get_locale();
        if let Some(root) = faq_ref.get() {
            // Wait for translated children to update before reading their text.
            if let Ok(frame) = request_animation_frame_with_handle(move || {
                let count = filter_faq(&root, &query);
                results.set((!query.is_empty()).then_some(count));
            }) {
                on_cleanup(move || frame.cancel());
            }
        }
    });
    let rel = "external noopener";
    let target = "_blank";
    let link_class = "ui-text-link";
    view! {
        <PageShell>
            <PageHeader
                title=move || { t_string!(i18n, faq.title) }
                subtitle=move || { t_string!(i18n, faq.subtitle) }
            />
            <div class="space-y-2">
                <div class="relative">
                    <Icon
                        icon=icondata_lu::LuSearch
                        attr:class="absolute left-3 top-1/2 -translate-y-1/2 size-5 pointer-events-none text-gray-500"
                        attr:aria-hidden="true"
                    />
                    <input
                        node_ref=input_ref
                        type="search"
                        aria-label=move || t_string!(i18n, faq.search.label)
                        class="pr-12 pl-10 ui-field-input [&::-webkit-search-cancel-button]:hidden"
                        placeholder=move || t_string!(i18n, faq.search.placeholder)
                        bind:value=query
                    />
                    <Show when=move || !query.get().trim().is_empty()>
                        <button
                            type="button"
                            class="flex absolute inset-y-0 right-0 justify-center items-center w-11"
                            aria-label=move || t_string!(i18n, common.clear_search)
                            on:click=move |_| {
                                query.set(String::new());
                                if let Some(input) = input_ref.get() {
                                    let _ = input.focus();
                                }
                            }
                        >
                            <Icon
                                icon=icondata_lu::LuX
                                attr:class="size-5"
                                attr:aria-hidden="true"
                            />
                        </button>
                    </Show>
                </div>
                <p class="text-sm text-gray-600 dark:text-gray-300" role="status">
                    {move || {
                        results
                            .get()
                            .map(|count| {
                                if count == 0 {
                                    t_string!(i18n, faq.search.no_results).to_string()
                                } else {
                                    t_string!(i18n, faq.search.results, count = count).to_string()
                                }
                            })
                    }}
                </p>
            </div>
            <div node_ref=faq_ref class="flex flex-col gap-6">
                <FaqSection
                    title=move || { t_string!(i18n, faq.sections.getting_started) }
                    icon=icondata_lu::LuBookOpen
                >
                    <FaqItem question=move || { t_string!(i18n, faq.what_is_hive.question) }>
                        <p>
                            {t!(
                                i18n, faq.what_is_hive.answer,
                                                    < gen42_link > =
                                                    <a href=GEN42_URL rel=rel target=target class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.hive_rules.question) }>
                        <p class="whitespace-pre-line">
                            {t!(
                                i18n, faq.hive_rules.answer,
                                                    < rules_link > =
                                                    <a href="https://hivegame.com/download/rules.pdf" rel=rel target=target class=link_class/>,
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
                        <p class="whitespace-pre-line">
                            {t!(i18n, faq.can_i_play_base_game.answer)}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.signup_required.question) }>
                        <p>
                            {t!(
                                i18n, faq.signup_required.answer, < register_link > = <a href="/register" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.mobile_friendly.question) }>
                        <p>{t!(i18n, faq.mobile_friendly.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.mobile_app.question) }>
                        <p>{t!(i18n, faq.mobile_app.answer)}</p>
                    </FaqItem>
                </FaqSection>

                <FaqSection
                    title=move || { t_string!(i18n, faq.sections.game_settings) }
                    icon=icondata_lu::LuSlidersHorizontal
                >
                    <FaqItem question=move || { t_string!(i18n, faq.start_game.question) }>
                        <p>
                            {t!(
                                i18n, faq.start_game.answer, < button_label > = <strong/>, < home_link > = <a href="/" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.play_specific_players.question)
                    }>
                        <p class="whitespace-pre-line">
                            {t!(
                                i18n, faq.play_specific_players.answer, < button_label > = <strong/>, < home_link > = <a href="/" class=link_class/>
                            )}
                            <Icon
                                icon=icondata_ri::RiSwordOthersLine
                                attr:class="inline-block align-text-bottom size-5"
                                attr:aria-hidden="true"
                            />
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.time_controls.question) }>
                        <p class="whitespace-pre-line">{t!(i18n, faq.time_controls.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.realtime_vs_correspondence.question)
                    }>
                        <p>{t!(i18n, faq.realtime_vs_correspondence.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.private_vs_public.question) }>
                        <p>
                            {t!(
                                i18n, faq.private_vs_public.answer, < home_link > = <a href="/" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.rating_range.question) }>
                        <p>{t!(i18n, faq.rating_range.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.color_selection.question) }>
                        <p>{t!(i18n, faq.color_selection.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.move_confirmation.question) }>
                        <p class="whitespace-pre-line">
                            {t!(
                                i18n, faq.move_confirmation.answer,
                                    < settings_link > = <a href="/config" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.how_to_check_stack.question) }>
                        <p>{t!(i18n, faq.how_to_check_stack.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.game_controls.question) }>
                        <p>{t!(i18n, faq.game_controls.answer)}</p>
                        <ul class="mt-3 space-y-3">
                            <li>
                                <Icon
                                    icon=icondata_fa::FaHandshakeSimpleSolid
                                    attr:class="inline-block mr-1 align-text-bottom size-5"
                                    attr:aria-hidden="true"
                                />
                                {t!(i18n, faq.game_controls.draw)}
                            </li>
                            <li>
                                <Icon
                                    icon=icondata_ai::AiFlagOutlined
                                    attr:class="inline-block mr-1 align-text-bottom size-5"
                                    attr:aria-hidden="true"
                                />
                                {t!(i18n, faq.game_controls.resign)}
                            </li>
                            <li>
                                <Icon
                                    icon=icondata_bi::BiUndoRegular
                                    attr:class="inline-block mr-1 align-text-bottom size-5"
                                    attr:aria-hidden="true"
                                />
                                {t!(i18n, faq.game_controls.takeback)}
                            </li>
                        </ul>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.resign_ok.question) }>
                        <p>{t!(i18n, faq.resign_ok.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.spectate_games.question) }>
                        <p>
                            {t!(
                                i18n, faq.spectate_games.answer, < home_link > = <a href="/" class=link_class/>
                            )}
                        </p>
                    </FaqItem>
                </FaqSection>

                <FaqSection
                    title=move || { t_string!(i18n, faq.sections.ratings) }
                    icon=icondata_lu::LuChartNoAxesCombined
                >
                    <FaqItem question=move || { t_string!(i18n, faq.casual_vs_rated.question) }>
                        <p>{t!(i18n, faq.casual_vs_rated.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.rating_system.question) }>
                        <p class="whitespace-pre-line">{t!(i18n, faq.rating_system.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.different_ratings.question) }>
                        <p class="whitespace-pre-line">{t!(i18n, faq.different_ratings.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.why_elo_has_questionmark.question)
                    }>
                        <p class="whitespace-pre-line">
                            {t!(i18n, faq.why_elo_has_questionmark.answer)}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.why_not_in_top_players.question)
                    }>
                        <p class="whitespace-pre-line">
                            {t!(
                                i18n, faq.why_not_in_top_players.answer,
                                    < leaderboard_link > = <a href="/top_players" class=link_class/>
                            )}
                        </p>
                    </FaqItem>
                </FaqSection>

                <FaqSection
                    title=move || { t_string!(i18n, common.tournaments) }
                    icon=icondata_lu::LuTrophy
                >
                    <FaqItem question=move || {
                        t_string!(i18n, faq.can_i_set_up_a_tournament.question)
                    }>
                        <p>
                            {t!(
                                i18n, faq.can_i_set_up_a_tournament.answer, < create_link > = <a href="/tournaments/create" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_entry.question) }>
                        <p>{t!(i18n, faq.tournament_entry.answer)}</p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_duration.question) }>
                        <p>
                            {t!(
                                i18n, faq.tournament_duration.answer, < tournaments_link > = <a href="/tournaments" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_stats.question) }>
                        <p class="whitespace-pre-line">
                            {t!(
                                i18n, faq.tournament_stats.answer,
                                    < tournaments_link > = <a href="/tournaments" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_terms.question) }>
                        <p>{t!(i18n, faq.tournament_terms.answer)}</p>
                        <dl class="mt-3 space-y-3">
                            {criterion_term(RoundRobin::DirectEncounter(Default::default()))}
                            {criterion_term(RoundRobin::GamesWonWithBlack)}
                            {criterion_term(RoundRobin::SonnebornBerger(Default::default()))}
                            {criterion_term(Swiss::Buchholz(Default::default()))}
                            {criterion_term(RoundRobin::Progressive(Default::default()))}
                            {glossary_term(
                                t!(i18n, tournaments.tiebreakers.names.koya),
                                t!(i18n, faq.tournament_terms.koya_answer),
                            )} {criterion_term(RoundRobin::NumberOfWins)}
                            {criterion_term(RoundRobin::GamesWon)}
                            {criterion_term(Swiss::NumberOfWins)}
                            {criterion_term(RoundRobin::MatchesWon)}
                            {glossary_term(
                                t!(i18n, tournaments.tiebreakers.names.game_points),
                                t!(i18n, tournaments.tiebreakers.game_points),
                            )}
                            {glossary_term(
                                t!(i18n, tournaments.tiebreakers.names.match_points),
                                t!(i18n, tournaments.tiebreakers.match_points),
                            )}
                            {criterion_term(
                                RoundRobin::TournamentPairingNumber(
                                    TournamentPairingNumberOrder::Ascending,
                                ),
                            )}
                            {criterion_term(
                                RoundRobin::TournamentPairingNumber(
                                    TournamentPairingNumberOrder::Descending,
                                ),
                            )}
                            {glossary_term(
                                t!(i18n, faq.tournament_terms.cut_name),
                                t!(i18n, faq.tournament_terms.cut_answer),
                            )}
                        </dl>
                    </FaqItem>
                </FaqSection>

                <FaqSection
                    title=move || { t_string!(i18n, faq.sections.account_configuration) }
                    icon=icondata_lu::LuUserRound
                >
                    <FaqItem question=move || { t_string!(i18n, faq.profile_navigation.question) }>
                        <p>
                            {t!(
                                i18n, faq.profile_navigation.answer, < profile_link > = <a href="/@/me" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.change_password.question) }>
                        <p>
                            {t!(
                                i18n, faq.change_password.answer,
                                                    < edit_link > =
                                                    <a href="/account" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || {
                        t_string!(i18n, faq.how_to_recover_password.question)
                    }>
                        <p>
                            {t!(
                                i18n, faq.how_to_recover_password.answer, < reset_link > = <a href="/forgot-password" class=link_class/>,
        < login_link > = <a href="/login" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.tournament_games.question) }>
                        <p>
                            {t!(
                                i18n, faq.tournament_games.answer, < profile_link > = <a href="/@/me" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.turn_alerts.question) }>
                        <p>
                            {t!(
                                i18n, faq.turn_alerts.answer, < profile_link > = <a href="/@/me" class=link_class/>,
        < notifications_link > = <a href="/notifications" class=link_class/>
                            )}
                        </p>
                    </FaqItem>
                    <FaqItem question=move || { t_string!(i18n, faq.notification_events.question) }>
                        <p>
                            <Icon
                                icon=icondata_io::IoNotifications
                                attr:class="inline-block mr-1 align-text-bottom text-ladybug-red size-4"
                                attr:aria-hidden="true"
                            />
                            {t!(i18n, faq.notification_events.answer)}
                        </p>
                        <ul class="mt-2 space-y-1 list-disc list-inside">
                            <li>{t!(i18n, faq.notification_events.messages)}</li>
                            <li>{t!(i18n, faq.notification_events.challenges)}</li>
                            <li>{t!(i18n, faq.notification_events.invitations)}</li>
                            <li>{t!(i18n, faq.notification_events.status)}</li>
                            <li>{t!(i18n, faq.notification_events.pairings)}</li>
                            <li>{t!(i18n, faq.notification_events.schedules)}</li>
                        </ul>
                        <p class="mt-3">
                            {t!(
                                i18n, faq.notification_events.settings, < notifications_link > = <a href="/notifications" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                </FaqSection>

                <FaqSection
                    title=move || { t_string!(i18n, faq.sections.more_features) }
                    icon=icondata_lu::LuGamepad2
                >
                    <FaqItem question=move || { t_string!(i18n, faq.get_better.question) }>
                        <p>
                            {t!(
                                i18n, faq.get_better.answer,
                                                    < resources_link > =
                                                    <a href="/resources" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.play_bots.question) }>
                        <p>
                            {t!(
                                i18n, faq.play_bots.answer, < button_label > = <strong/>, < home_link > = <a href="/" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.game_analysis.question) }>
                        <p>
                            {t!(
                                i18n, faq.game_analysis.answer,
                                < analysis_link > = <a href="/analysis" class=link_class/>,
                                < microscope_icon > = |children: ChildrenFn| view! {
                                    <span class="inline-flex gap-1 items-center align-bottom">
                                        {children()}
                                        <Icon icon=icondata_tb::TbMicroscopeOutline attr:class="size-5" attr:aria-hidden="true" />
                                    </span>
                                }
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.game_archive.question) }>
                        <p>
                            {t!(
                                i18n, faq.game_archive.answer, < archive_link > = <a href="/archive" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.opening_statistics.question) }>
                        <p>
                            {t!(
                                i18n, faq.opening_statistics.answer, < analysis_link > = <a href="/analysis" class=link_class/>
                            )}
                        </p>
                    </FaqItem>
                </FaqSection>

                <FaqSection
                    title=move || { t_string!(i18n, faq.sections.community_support) }
                    icon=icondata_lu::LuUsers
                >
                    <FaqItem question=move || { t_string!(i18n, faq.chat_basics.question) }>
                        <p>
                            {t!(
                                i18n, faq.chat_basics.answer,
                                < messages_link > = <a href="/message" class=link_class/>,
                                < message_icon > = |children: ChildrenFn| view! {
                                    <span class="inline-flex gap-1 items-center align-bottom">
                                        {children()}
                                        <Icon icon=icondata_bi::BiChatRegular attr:class="size-5" attr:aria-hidden="true" />
                                    </span>
                                }
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.community_forum.question) }>
                        <p>
                            {t!(
                                i18n, faq.community_forum.answer,
                                                    < discord_link > =
                                                    <a href=WORLD_HIVE_DISCORD_URL rel=rel target=target class=link_class/>,
                                    < resources_link > = <a href="/resources" class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.report_bugs.question) }>
                        <p>
                            {t!(
                                i18n, faq.report_bugs.answer,
                                                    < discord_link > =
                                                    <a href=HIVEGAME_DISCORD_URL rel=rel target=target class=link_class/>,
                                                    < issues_link > = <a href=GITHUB_ISSUES_URL rel=rel target=target class=link_class/>
                            )}
                        </p>
                    </FaqItem>

                    <FaqItem question=move || { t_string!(i18n, faq.opponent_is_abusive.question) }>
                        <p>{t!(i18n, faq.opponent_is_abusive.answer)}</p>
                    </FaqItem>
                </FaqSection>

                <FaqSection
                    title=move || { t_string!(i18n, faq.sections.donating) }
                    icon=icondata_lu::LuHeart
                >
                    <FaqItem question=move || { t_string!(i18n, faq.how_to_help.question) }>
                        <ul class="list-disc list-inside">
                            <li>{t!(i18n, faq.how_to_help.answers.item1)}</li>
                            <li>
                                {t!(
                                    i18n, faq.how_to_help.answers.item2,
                                                            < github_link > =
                                                            <a href=GITHUB_URL rel=rel target=target class=link_class/>
                                )}
                            </li>
                            <li>
                                {t!(
                                    i18n, faq.how_to_help.answers.item3, < donate_link > = <a href="/donate" class=link_class/>
                                )}
                            </li>
                        </ul>
                    </FaqItem>

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
                                                    <a href="/donate" class=link_class/>
                            )}
                        </p>
                    </FaqItem>
                </FaqSection>
            </div>
        </PageShell>
    }
}
