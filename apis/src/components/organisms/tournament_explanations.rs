use crate::{
    common::{
        game_point_presentation,
        standings_value_text,
        ScorePresentation,
        StandingsCriterionChoice,
    },
    components::molecules::time_row::TimeRow,
    i18n::*,
};
use leptos::prelude::*;
use leptos_i18n::I18nContext;
use shared_types::tournament::{
    elimination::{ClinchPolicy, SetLimit, Stage, Topology},
    round_robin::{
        Config as RoundRobinConfig,
        Criterion as RoundRobinCriterion,
        PrimaryScore as RoundRobinPrimaryScore,
    },
    standings::Value,
    swiss::{
        Criterion as SwissCriterion,
        PrimaryScore as DoubleSwissPrimaryScore,
        System as SwissSystem,
    },
    Format,
    FormatConfig,
    ReleasePolicy,
    Score,
};
use tournamint::arena::ArenaConfig as NativeArenaConfig;

fn release_policy_text(i18n: I18nContext<Locale, I18nKeys>, policy: ReleasePolicy) -> String {
    match policy {
        ReleasePolicy::FullyUnlocked => {
            t_string!(i18n, tournaments.creation.faq.release_all).to_string()
        }
        ReleasePolicy::SequentialPerMatchup => {
            t_string!(i18n, tournaments.creation.faq.release_sequential).to_string()
        }
    }
}

#[derive(Clone)]
struct FormatFaqContent {
    introduction: String,
    format_entries: Vec<(String, String)>,
    entries: Vec<(String, String)>,
}

fn swiss_common_faq(i18n: I18nContext<Locale, I18nKeys>) -> Vec<(String, String)> {
    vec![
        (
            t_string!(i18n, tournaments.creation.faq.pairings_question).to_string(),
            t_string!(i18n, tournaments.creation.faq.swiss_pairings).to_string(),
        ),
        (
            t_string!(i18n, tournaments.creation.faq.odd_field_question).to_string(),
            t_string!(i18n, tournaments.creation.faq.swiss_odd_field).to_string(),
        ),
        (
            t_string!(i18n, tournaments.creation.faq.winner_question).to_string(),
            t_string!(i18n, tournaments.creation.faq.swiss_winner).to_string(),
        ),
    ]
}

fn format_faq_content(i18n: I18nContext<Locale, I18nKeys>, format: Format) -> FormatFaqContent {
    match format {
        Format::Arena => FormatFaqContent {
            introduction: t_string!(i18n, tournaments.creation.faq.arena_intro).to_string(),
            format_entries: vec![],
            entries: vec![],
        },

        Format::RoundRobin => FormatFaqContent {
            introduction: t_string!(i18n, tournaments.creation.faq.round_robin_intro).to_string(),
            format_entries: vec![],
            entries: vec![
                (
                    t_string!(i18n, tournaments.creation.faq.pairing_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.round_robin_pairing).to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.availability_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.round_robin_availability).to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.winner_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.round_robin_winner).to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.end_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.round_robin_end).to_string(),
                ),
            ],
        },

        Format::Swiss => FormatFaqContent {
            introduction: t_string!(i18n, tournaments.creation.faq.swiss_intro).to_string(),
            format_entries: vec![(
                t_string!(i18n, tournaments.creation.faq.knockout_question).to_string(),
                t_string!(i18n, tournaments.creation.faq.swiss_knockout).to_string(),
            )],
            entries: swiss_common_faq(i18n),
        },

        Format::DoubleSwiss => FormatFaqContent {
            introduction: t_string!(i18n, tournaments.creation.faq.double_swiss_intro).to_string(),
            format_entries: vec![(
                t_string!(i18n, tournaments.creation.faq.scores_question).to_string(),
                t_string!(i18n, tournaments.creation.faq.double_swiss_scores).to_string(),
            )],
            entries: swiss_common_faq(i18n),
        },

        Format::SingleElimination => FormatFaqContent {
            introduction: t_string!(i18n, tournaments.creation.faq.single_elimination_intro)
                .to_string(),
            format_entries: vec![],
            entries: vec![
                (
                    t_string!(i18n, tournaments.creation.faq.seeding_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.bracket_seeding).to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.match_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.elimination_match).to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.third_place_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.elimination_third_place).to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.end_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.single_elimination_end).to_string(),
                ),
            ],
        },

        Format::DoubleElimination => FormatFaqContent {
            introduction: t_string!(i18n, tournaments.creation.faq.double_elimination_intro)
                .to_string(),
            format_entries: vec![],
            entries: vec![
                (
                    t_string!(i18n, tournaments.creation.faq.brackets_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.double_elimination_brackets)
                        .to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.seeding_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.bracket_seeding).to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.match_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.elimination_match).to_string(),
                ),
                (
                    t_string!(i18n, tournaments.creation.faq.end_question).to_string(),
                    t_string!(i18n, tournaments.creation.faq.double_elimination_end).to_string(),
                ),
            ],
        },
    }
}

fn arena_faq_entries(
    i18n: I18nContext<Locale, I18nKeys>,
    rules: NativeArenaConfig,
) -> Vec<(String, String)> {
    let scoring = if rules.scoring.fire {
        t_string!(i18n, tournaments.creation.faq.arena_fire_scoring).to_string()
    } else {
        t_string!(i18n, tournaments.creation.faq.arena_scoring).to_string()
    };
    let berserk = match rules.scoring.berserk {
        Some(policy) => {
            let bonus = if policy.win_bonus.value() == 1 {
                t_string!(i18n, tournaments.creation.faq.berserk_bonus_one).to_string()
            } else {
                t_string!(
                    i18n,
                    tournaments.creation.faq.berserk_bonus,
                    points = policy.win_bonus.value()
                )
                .to_string()
            };
            t_string!(
                i18n,
                tournaments.creation.faq.berserk_help,
                bonus = bonus,
                moves = policy.minimum_bonus_plies
            )
            .to_string()
        }
        None => t_string!(i18n, tournaments.creation.faq.berserk_disabled).to_string(),
    };
    vec![
        (
            t_string!(i18n, tournaments.creation.faq.scores_question).to_string(),
            scoring,
        ),
        (t_string!(i18n, game.arena.berserk).to_string(), berserk),
        (
            t_string!(i18n, tournaments.creation.faq.pairing_question).to_string(),
            t_string!(i18n, tournaments.creation.faq.arena_pairing).to_string(),
        ),
        (
            t_string!(i18n, tournaments.creation.faq.end_question).to_string(),
            t_string!(i18n, tournaments.creation.faq.arena_end).to_string(),
        ),
    ]
}

#[component]
pub fn TournamentFormatFaq(
    format: Signal<Format>,
    #[prop(optional)] arena_rules: Option<Signal<NativeArenaConfig>>,
) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        {move || {
            let format = format.get();
            let content = format_faq_content(i18n, format);
            let entries = if format == Format::Arena {
                arena_faq_entries(i18n, arena_rules.map(|rules| rules.get()).unwrap_or_default())
            } else {
                content.format_entries.iter().chain(content.entries.iter()).cloned().collect()
            };
            view! {
                <div class="mx-auto space-y-5 w-full max-w-3xl text-gray-700 dark:text-gray-300">
                    <p class="text-sm">{content.introduction}</p>
                    <div class="space-y-4">
                        {entries
                            .into_iter()
                            .map(|(question, answer)| {
                                view! {
                                    <section class="space-y-1">
                                        <h3 class="font-semibold text-gray-900 dark:text-gray-100">
                                            {question}
                                        </h3>
                                        <p class="text-sm leading-relaxed">{answer}</p>
                                    </section>
                                }
                            })
                            .collect_view()}
                    </div>
                </div>
            }
        }}
    }
}

pub(crate) fn phase_text(games: u16, limit: SetLimit, clinch: ClinchPolicy) -> String {
    let i18n = use_i18n();

    let limit = match limit {
        SetLimit::AtMost(sets) => {
            t_string!(i18n, tournaments.ui.up_to_sets, sets = sets).to_string()
        }
        SetLimit::UntilDecisive => t_string!(
            i18n,
            tournaments.creation.rules_summary.phase_until_decisive
        )
        .to_string(),
    };
    let clinch = match clinch {
        ClinchPolicy::PlayAll => t_string!(i18n, tournaments.ui.all_games_played).to_string(),
        ClinchPolicy::EarlyClinch => t_string!(i18n, tournaments.ui.ends_when_clinched).to_string(),
    };
    t_string!(
        i18n,
        tournaments.creation.rules_summary.phase,
        games = games,
        limit = limit,
        clinch = clinch
    )
    .to_string()
}

pub(crate) fn stage_plan_label(stage: Stage) -> String {
    let i18n = use_i18n();

    match stage {
        Stage::SingleRound { round_index } => t_string!(
            i18n,
            tournaments.view.bracket.stage.round,
            round = round_index + 1
        )
        .to_string(),
        Stage::SingleFinal => {
            t_string!(i18n, tournaments.view.bracket.stage.final_match).to_string()
        }
        Stage::Bronze => t_string!(i18n, tournaments.ui.third_place_match_title).to_string(),
        Stage::WinnersRound { round_index } => t_string!(
            i18n,
            tournaments.creation.rules_summary.upper_round,
            round = round_index + 1
        )
        .to_string(),
        Stage::LosersMinor { round_index } => t_string!(
            i18n,
            tournaments.creation.rules_summary.lower_round,
            round = round_index * 2 + 1
        )
        .to_string(),
        Stage::LosersMajor { round_index } => t_string!(
            i18n,
            tournaments.creation.rules_summary.lower_round,
            round = round_index * 2 + 2
        )
        .to_string(),
        Stage::GrandFinal => t_string!(i18n, tournaments.pairing.stage.grand_final).to_string(),
        Stage::Reset => t_string!(i18n, tournaments.pairing.stage.reset_final).to_string(),
    }
}

fn point_awards(configuration: &FormatConfig) -> Vec<String> {
    let i18n = use_i18n();
    let (game, match_points) = match configuration {
        FormatConfig::RoundRobin(configuration) => (
            configuration.rules.point_system,
            (configuration.rules.primary_score == RoundRobinPrimaryScore::MatchPoints
                || configuration
                    .rules
                    .standings
                    .criteria
                    .contains(&RoundRobinCriterion::MatchPoints))
            .then_some(configuration.rules.match_point_system),
        ),
        FormatConfig::Swiss(configuration) => (
            configuration.game_point_system,
            match configuration.system {
                SwissSystem::DoubleSwiss(system)
                    if system.primary_score == DoubleSwissPrimaryScore::MatchPoints
                        || configuration
                            .standings
                            .contains(&SwissCriterion::MatchPoints) =>
                {
                    Some(system.match_point_system)
                }
                _ => None,
            },
        ),
        _ => return Vec::new(),
    };
    let awards = |name: &str, points: [Score; 4], presentation| {
        let [win, draw, loss, forfeit] =
            points.map(|score| standings_value_text(Value::Score(score), presentation));

        t_string!(
            i18n,
            tournaments.creation.rules_summary.point_awards,
            name = name,
            win = win,
            draw = draw,
            loss = loss,
            forfeit = forfeit
        )
        .to_string()
    };

    let mut rows = vec![awards(
        t_string!(i18n, tournaments.tiebreakers.names.game_points),
        [game.win, game.draw, game.loss, game.forfeit_loss],
        game_point_presentation(game),
    )];
    if let Some(points) = match_points {
        rows.push(awards(
            t_string!(i18n, tournaments.tiebreakers.names.match_points),
            [points.win, points.draw, points.loss, points.forfeit_loss],
            ScorePresentation::MatchPoints,
        ));
    }
    rows
}

#[component]
pub fn TournamentFormatConfigurationDetails(configuration: FormatConfig) -> impl IntoView {
    let i18n = use_i18n();
    move || {
        match configuration.clone() {
        FormatConfig::RoundRobin(configuration) => {

            view! {
                <ul class="pl-5 space-y-1 text-sm list-disc">
                    <li>
                        {if configuration.rules.has_matches() {
                            t_string!(
                                i18n, tournaments.creation.rules_summary.match_per_opponent, games = configuration.rules.repeats
                            )
                                .to_string()
                        } else {
                            t_string!(
                                i18n, tournaments.creation.rules_summary.games_per_opponent, games = configuration.rules.repeats
                            )
                                .to_string()
                        }}
                    </li>
                    <li>{release_policy_text(i18n, configuration.release_policy)}</li>
                </ul>
            }
            .into_any()
        }
        FormatConfig::Swiss(configuration) => {
            let rounds = configuration
                .rounds
                .resolved_rounds()
                .map(|rounds| rounds.get());
            let double_swiss = match configuration.system {
                SwissSystem::DoubleSwiss(system) => Some((
                    system.primary_score,
                    configuration.double_swiss_release_policy,
                )),
                _ => None,
            };
            view! {
                <div class="space-y-2">
                    {rounds
                        .map(|rounds| {
                            view! {
                                <p class="text-sm">
                                    {t_string!(
                                        i18n, tournaments.creation.rounds_count, count = rounds
                                    )
                                        .to_string()}
                                </p>
                            }
                        })}
                    {double_swiss
                        .map(|(primary_score, release_policy)| {
                            let scoring = match primary_score {
                                DoubleSwissPrimaryScore::GamePoints => {
                                    t_string!(
                                        i18n, tournaments.creation.rules_summary.individual_game_points
                                    )
                                        .to_string()
                                }
                                DoubleSwissPrimaryScore::MatchPoints => {
                                    t_string!(
                                        i18n, tournaments.creation.rules_summary.two_game_match_points
                                    )
                                        .to_string()
                                }
                            };

                            view! {
                                <p class="text-sm">{scoring}</p>
                                <p class="text-sm">{release_policy_text(i18n, release_policy)}</p>
                            }
                        })}
                </div>
            }
            .into_any()
        }
        FormatConfig::Elimination(configuration) => {

            let topology = match configuration.topology {
                Topology::Single { bronze: true } => t_string!(i18n, tournaments.creation.rules_summary.single_elimination_bronze).to_string(),
                Topology::Single { bronze: false } => t_string!(i18n, tournaments.format.single_elimination).to_string(),
                Topology::Double => t_string!(i18n, tournaments.format.double_elimination).to_string(),
            };
            let phases = configuration
                .default_plan
                .phases
                .iter()
                .map(|phase| {
                    (
                        phase_text(phase.games_per_set, phase.set_limit, phase.clinch),
                        phase.clock,
                    )
                })
                .collect::<Vec<_>>();
            view! {
                <div class="space-y-2 text-sm">
                    <p>{topology}</p>
                    <ol class="pl-5 space-y-1 list-decimal">
                        {phases
                            .into_iter()
                            .map(|(phase, clock)| {
                                view! { <li>{phase}<TimeRow time_control=Some(clock) /></li> }
                            })
                            .collect_view()}
                    </ol>

                    <p>{t!(i18n, tournaments.creation.rules_summary.match_availability)}</p>
                    {(configuration.topology == Topology::Double)
                        .then(|| {
                            view! {
                                <p>
                                    {t!(i18n, tournaments.creation.rules_summary.reset_final_help)}
                                </p>
                            }
                        })}
                    {configuration
                        .stage_overrides
                        .into_iter()
                        .map(|override_| {
                            view! {
                                <div class="pt-2 border-t border-black/10 dark:border-white/10">
                                    <strong>{stage_plan_label(override_.stage)}</strong>
                                    <ol class="pl-5 list-decimal">
                                        {override_
                                            .plan
                                            .phases
                                            .into_iter()
                                            .map(|phase| {
                                                view! {
                                                    <li>
                                                        {phase_text(
                                                            phase.games_per_set,
                                                            phase.set_limit,
                                                            phase.clinch,
                                                        )}<TimeRow time_control=Some(phase.clock) />
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                    </ol>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            }
            .into_any()
        }
        FormatConfig::Arena(_) => ().into_any(),
    }
    }
}

pub(crate) fn round_robin_criterion_name(
    i18n: I18nContext<Locale, I18nKeys>,
    configuration: &RoundRobinConfig,
    criterion: RoundRobinCriterion,
) -> String {
    if criterion == RoundRobinCriterion::PrimaryScore {
        return match configuration.rules.primary_score {
            RoundRobinPrimaryScore::GamePoints => RoundRobinCriterion::GamePoints.name(i18n),
            RoundRobinPrimaryScore::MatchPoints => RoundRobinCriterion::MatchPoints.name(i18n),
        };
    }
    criterion.name(i18n)
}

pub(crate) fn round_robin_criterion_explanation(
    i18n: I18nContext<Locale, I18nKeys>,
    configuration: &RoundRobinConfig,
    criterion: RoundRobinCriterion,
) -> String {
    let criterion = if criterion == RoundRobinCriterion::PrimaryScore {
        match configuration.rules.primary_score {
            RoundRobinPrimaryScore::GamePoints => RoundRobinCriterion::GamePoints,
            RoundRobinPrimaryScore::MatchPoints => RoundRobinCriterion::MatchPoints,
        }
    } else {
        criterion
    };

    if criterion == RoundRobinCriterion::MatchPoints {
        return t_string!(
            i18n,
            tournaments.creation.rules_summary.round_robin_match_points,
            games = configuration.rules.repeats
        )
        .to_string();
    }
    let explanation = criterion.explanation(i18n);
    if matches!(
        criterion,
        RoundRobinCriterion::DirectEncounter(_)
            | RoundRobinCriterion::SonnebornBerger(_)
            | RoundRobinCriterion::Koya(_)
            | RoundRobinCriterion::Progressive(_)
    ) {
        let basis = match configuration.rules.primary_score {
            RoundRobinPrimaryScore::GamePoints => {
                t_string!(i18n, tournaments.creation.rules_summary.game_points_lower).to_string()
            }
            RoundRobinPrimaryScore::MatchPoints => {
                t_string!(i18n, tournaments.creation.rules_summary.match_points_lower).to_string()
            }
        };

        return t_string!(
            i18n,
            tournaments.creation.rules_summary.criterion_primary_score,
            explanation = explanation,
            basis = basis
        )
        .to_string();
    }
    explanation
}

pub(crate) fn swiss_criterion_name(
    i18n: I18nContext<Locale, I18nKeys>,
    configuration: &shared_types::tournament::swiss::Config,
    criterion: SwissCriterion,
) -> String {
    if criterion != SwissCriterion::PrimaryScore {
        return criterion.name(i18n);
    }
    match configuration.system {
        SwissSystem::DoubleSwiss(system)
            if system.primary_score == DoubleSwissPrimaryScore::MatchPoints =>
        {
            t_string!(i18n, tournaments.tiebreakers.names.match_points).to_string()
        }
        SwissSystem::DoubleSwiss(_) | SwissSystem::Dutch(_) => {
            t_string!(i18n, tournaments.tiebreakers.names.game_points).to_string()
        }
        _ => unreachable!("unsupported Swiss system passed creation validation"),
    }
}

pub(crate) fn swiss_criterion_explanation(
    i18n: I18nContext<Locale, I18nKeys>,
    configuration: &shared_types::tournament::swiss::Config,
    criterion: SwissCriterion,
) -> String {
    if criterion != SwissCriterion::PrimaryScore {
        return criterion.explanation(i18n);
    }
    match configuration.system {
        SwissSystem::DoubleSwiss(system)
            if system.primary_score == DoubleSwissPrimaryScore::MatchPoints =>
        {
            t_string!(i18n, tournaments.tiebreakers.match_points).to_string()
        }
        SwissSystem::DoubleSwiss(_) | SwissSystem::Dutch(_) => {
            t_string!(i18n, tournaments.tiebreakers.game_points).to_string()
        }
        _ => unreachable!("unsupported Swiss system passed creation validation"),
    }
}

#[component]
pub(crate) fn StandingsCriteriaRules(configuration: FormatConfig) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        {move || {
            let summary = match &configuration {
                FormatConfig::RoundRobin(configuration) => {
                    Some((
                        (
                            round_robin_criterion_name(
                                i18n,
                                configuration,
                                RoundRobinCriterion::PrimaryScore,
                            ),
                            round_robin_criterion_explanation(
                                i18n,
                                configuration,
                                RoundRobinCriterion::PrimaryScore,
                            ),
                        ),
                        configuration
                            .rules
                            .standings
                            .criteria
                            .iter()
                            .copied()
                            .filter(|criterion| *criterion != RoundRobinCriterion::PrimaryScore)
                            .map(|criterion| (
                                round_robin_criterion_name(i18n, configuration, criterion),
                                round_robin_criterion_explanation(i18n, configuration, criterion),
                            ))
                            .collect::<Vec<_>>(),
                    ))
                }
                FormatConfig::Swiss(configuration) => {
                    Some((
                        (
                            swiss_criterion_name(i18n, configuration, SwissCriterion::PrimaryScore),
                            swiss_criterion_explanation(
                                i18n,
                                configuration,
                                SwissCriterion::PrimaryScore,
                            ),
                        ),
                        configuration
                            .standings
                            .iter()
                            .copied()
                            .filter(|criterion| *criterion != SwissCriterion::PrimaryScore)
                            .map(|criterion| {
                                (
                                    swiss_criterion_name(i18n, configuration, criterion),
                                    swiss_criterion_explanation(i18n, configuration, criterion),
                                )
                            })
                            .collect::<Vec<_>>(),
                    ))
                }
                FormatConfig::Elimination(_) | FormatConfig::Arena(_) => None,
            };
            let Some((primary, tiebreaks)) = summary else {
                return ().into_any();
            };
            view! {
                <div class="mt-3 space-y-2">

                    <h3 class="text-sm font-bold">{t!(i18n, tournaments.creation.scoring)}</h3>
                    <div class="p-2 ui-setting-group">
                        <strong>{primary.0}</strong>
                        <p class="text-sm text-gray-600 dark:text-gray-300">{primary.1}</p>
                        {point_awards(&configuration)
                            .into_iter()
                            .map(|awards| view! { <p class="text-sm">{awards}</p> })
                            .collect_view()}
                    </div>
                    {(!tiebreaks.is_empty())
                        .then(|| {
                            view! {
                                <h3 class="text-sm font-bold">
                                    {t!(i18n, tournaments.creation.rules_summary.tiebreaks)}
                                </h3>
                                <ol class="space-y-2">
                                    {tiebreaks
                                        .into_iter()
                                        .enumerate()
                                        .map(|(index, (name, explanation))| {
                                            view! {
                                                <li class="p-2 ui-setting-group">
                                                    <div class="flex flex-wrap gap-2 items-baseline">
                                                        <span class="text-xs font-bold uppercase text-pillbug-teal">
                                                            {format!("{}.", index + 1)}
                                                        </span>
                                                        <strong>{name}</strong>
                                                    </div>
                                                    <p class="text-sm text-gray-600 dark:text-gray-300">
                                                        {explanation}
                                                    </p>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ol>
                            }
                        })}
                </div>
            }
                .into_any()
        }}
    }
}
