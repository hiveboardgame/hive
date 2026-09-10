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

fn release_policy_text(policy: ReleasePolicy) -> &'static str {
    // TODO: i18n once copy is approved.
    match policy {
        ReleasePolicy::FullyUnlocked => "All games are available together.",
        ReleasePolicy::SequentialPerMatchup => {
            "The next game becomes available after the previous game is resolved."
        }
    }
}

#[derive(Clone, Copy)]
struct FormatFaqContent {
    title: &'static str,
    introduction: &'static str,
    format_entries: &'static [(&'static str, &'static str)],
    entries: &'static [(&'static str, &'static str)],
}

// TODO: i18n once copy is approved.
const SWISS_COMMON_FAQ: &[(&str, &str)] = &[
    (
        "How are pairings chosen?",
        "Players near the same score are paired while rematches are avoided whenever a legal pairing is available.",
    ),
    (
        "What happens with an odd field?",
        "One player receives the configured bye for that round and returns to the pairing pool next round.",
    ),
    (
        "How is the winner decided?",
        "After the planned rounds, or earlier if no legal pairing remains, players are ranked by their standings score and then the tournament's tiebreaks.",
    ),
];

fn format_faq_content(format: Format) -> FormatFaqContent {
    match format {
        // TODO: i18n once copy is approved.
        Format::Arena => FormatFaqContent {
            title: "Arena tournament FAQ",
            introduction: "Play as many games as you can before the Arena clock expires.",
            format_entries: &[],
            entries: &[
                (
                    "How does the pairing work?",
                    "Available players are paired repeatedly, usually against someone near their place in the standings. You may not face every other player.",
                ),
                (
                    "How are scores calculated?",
                    "Normally, a win is worth 2 points, a draw 1, and a loss 0. Short or repeated draws may score 0. After two consecutive wins, the next eligible win or draw is worth double; any non-win then ends the streak.",
                ),
                (
                    "What is Berserk?",
                    "Before your first move, Berserk removes the increment and usually half your starting time. A qualifying win earns one extra Arena point.",
                ),
                (
                    "How does it end?",
                    "When the Arena clock reaches zero, the standings freeze. Games already in progress can finish, but no longer affect the tournament.",
                ),
            ],
        },
        // TODO: i18n once copy is approved.
        Format::RoundRobin => FormatFaqContent {
            title: "Round Robin tournament FAQ",
            introduction: "Every entrant plays every other entrant the configured number of times.",
            format_entries: &[],
            entries: &[
                (
                    "How does the pairing work?",
                    "The complete field is scheduled, so every player faces the same set of opponents.",
                ),
                (
                    "When can I play my games?",
                    "Depending on the tournament settings, games are available together or one at a time against each opponent.",
                ),
                (
                    "How is the winner decided?",
                    "Players are ranked by the configured game or match points. With 2, 4, or 6 games per opponent, all those games form one match. If scores are tied, the tournament's selected tiebreaks are applied in order.",
                ),
                (
                    "How does it end?",
                    "The tournament finishes after every scheduled game has been resolved and the final standings can be calculated.",
                ),
            ],
        },
        // TODO: i18n once copy is approved.
        Format::Swiss => FormatFaqContent {
            title: "Swiss tournament FAQ",
            introduction: "Play one game per round against an opponent near your current score.",
            format_entries: &[
                (
                    "Is Swiss a knockout?",
                    "No. A loss changes your score and future pairings, but you remain in the tournament.",
                ),
            ],
            entries: SWISS_COMMON_FAQ,
        },
        // TODO: i18n once copy is approved.
        Format::DoubleSwiss => FormatFaqContent {
            title: "Double Swiss tournament FAQ",
            introduction: "Each round pairs you with one opponent for two games instead of one.",
            format_entries: &[
                (
                    "How are scores calculated?",
                    "The tournament ranks players by game points or by points from each two-game match. The other score can be selected as a tiebreak. The scoring mode and tiebreak order are shown in the tournament details.",
                ),
            ],
            entries: SWISS_COMMON_FAQ,
        },
        // TODO: i18n once copy is approved.
        Format::SingleElimination => FormatFaqContent {
            title: "Single Elimination tournament FAQ",
            introduction: "Win your match to stay in title contention. A loss ends that path, although a semifinal loser may still play a configured third-place match.",
            format_entries: &[],
            entries: &[
                (
                    "How is the bracket seeded?",
                    "The default bracket separates players by rating seed. Before an organizer starts the tournament, they can swap players between the fixed bracket positions, including those receiving byes. Seed numbers stay tied to the rating order frozen when bracket setup opens. Scheduled starts use the default bracket.",
                ),
                (
                    "What counts as one match?",
                    "A match follows the configured series plan and may contain several games or sets. It can end early once a player has clinched it.",
                ),
                (
                    "Is there a third-place match?",
                    "Only when the tournament enables one. Otherwise the semifinal losers finish in the same elimination tier.",
                ),
                (
                    "How does it end?",
                    "The winner of the final is the tournament champion.",
                ),
            ],
        },
        // TODO: i18n once copy is approved.
        Format::DoubleElimination => FormatFaqContent {
            title: "Double Elimination tournament FAQ",
            introduction: "One loss sends you to the lower bracket. A second loss eliminates you.",
            format_entries: &[],
            entries: &[
                (
                    "How do the brackets work?",
                    "Undefeated players remain in the upper bracket. After a loss, they continue through the lower bracket for as long as they keep winning.",
                ),
                (
                    "How is the bracket seeded?",
                    "The default bracket separates players by rating seed. Before an organizer starts the tournament, they can swap players between the fixed bracket positions, including those receiving byes. Seed numbers stay tied to the rating order frozen when bracket setup opens. Scheduled starts use the default bracket.",
                ),
                (
                    "What counts as one match?",
                    "A match follows the configured series plan and may contain several games or sets. It can end early once a player has clinched it.",
                ),
                (
                    "How does it end?",
                    "The upper- and lower-bracket winners meet in the final. If the lower-bracket winner wins, they play a second final.",
                ),
            ],
        },
    }
}

#[component]
pub fn TournamentFormatFaq(format: Signal<Format>) -> impl IntoView {
    view! {
        {move || {
            let content = format_faq_content(format.get());
            view! {
                <div class="mx-auto space-y-5 w-full max-w-3xl text-gray-700 dark:text-gray-300">
                    <div class="space-y-1">
                        <h2 class="text-lg font-bold text-gray-900 dark:text-gray-100">
                            {content.title}
                        </h2>
                        <p class="text-sm">{content.introduction}</p>
                    </div>
                    <div class="space-y-4">
                        {content
                            .format_entries
                            .iter()
                            .chain(content.entries.iter())
                            .copied()
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
    // TODO: i18n once copy is approved.
    let limit = match limit {
        SetLimit::AtMost(sets) => format!("up to {sets} sets"),
        SetLimit::UntilDecisive => String::from("sets until decisive"),
    };
    let clinch = match clinch {
        ClinchPolicy::PlayAll => "all games played",
        ClinchPolicy::EarlyClinch => "ends when clinched",
    };
    format!("{games} games per set · {limit} · {clinch}")
}

fn stage_plan_label(stage: Stage) -> String {
    // TODO: i18n once copy is approved.
    match stage {
        Stage::SingleRound { round_index } => format!("Round {}", round_index + 1),
        Stage::SingleFinal => "Final".to_string(),
        Stage::Bronze => "Third-place match".to_string(),
        Stage::WinnersRound { round_index } => format!("Upper bracket round {}", round_index + 1),
        Stage::LosersMinor { round_index } => {
            format!("Lower bracket round {}", round_index * 2 + 1)
        }
        Stage::LosersMajor { round_index } => {
            format!("Lower bracket round {}", round_index * 2 + 2)
        }
        Stage::GrandFinal => "Final".to_string(),
        Stage::Reset => "Second final".to_string(),
    }
}

fn point_awards(configuration: &FormatConfig) -> Vec<String> {
    let (game, match_points) = match configuration {
        FormatConfig::RoundRobin(configuration) => (
            configuration.game_point_system,
            configuration
                .has_matches()
                .then_some(configuration.match_point_system),
        ),
        FormatConfig::Swiss(configuration) => (
            configuration.game_point_system,
            match configuration.system {
                SwissSystem::DoubleSwiss(system) => Some(system.match_point_system),
                _ => None,
            },
        ),
        _ => return Vec::new(),
    };
    let awards = |name: &str, points: [Score; 4], presentation| {
        let [win, draw, loss, forfeit] =
            points.map(|score| standings_value_text(Value::Score(score), presentation));
        // TODO: i18n once copy is approved.
        format!("{name}: win {win} · draw {draw} · loss {loss} · forfeit loss {forfeit}")
    };
    // TODO: i18n once copy is approved.
    let mut rows = vec![awards(
        "Game points",
        [game.win, game.draw, game.loss, game.forfeit_loss],
        game_point_presentation(game),
    )];
    if let Some(points) = match_points {
        // TODO: i18n once copy is approved.
        rows.push(awards(
            "Match points",
            [points.win, points.draw, points.loss, points.forfeit_loss],
            ScorePresentation::MatchPoints,
        ));
    }
    rows
}

#[component]
pub fn TournamentFormatConfigurationDetails(configuration: FormatConfig) -> impl IntoView {
    match configuration {
        FormatConfig::RoundRobin(configuration) => {
            // TODO: i18n once copy is approved.
            view! {
                <ul class="pl-5 space-y-1 text-sm list-disc">
                    <li>
                        {if configuration.has_matches() {
                            format!(
                                "One {}-game match against every opponent",
                                configuration.repeats.get(),
                            )
                        } else {
                            format!("{} games against every opponent", configuration.repeats.get())
                        }}
                    </li>
                    <li>{release_policy_text(configuration.release_policy)}</li>
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
                                // TODO: i18n once copy is approved.
                                <p class="text-sm">{format!("{rounds} rounds")}</p>
                            }
                        })}
                    {double_swiss
                        .map(|(primary_score, release_policy)| {
                            let scoring = match primary_score {
                                DoubleSwissPrimaryScore::GamePoints => "Individual game points",
                                DoubleSwissPrimaryScore::MatchPoints => "Two-game match points",
                            };
                            // TODO: i18n once copy is approved.
                            view! {
                                <p class="text-sm">{scoring}</p>
                                <p class="text-sm">{release_policy_text(release_policy)}</p>
                            }
                        })}
                </div>
            }
            .into_any()
        }
        FormatConfig::Elimination(configuration) => {
            // TODO: i18n once copy is approved.
            let topology = match configuration.topology {
                Topology::Single { bronze: true } => "Single elimination with a third-place match",
                Topology::Single { bronze: false } => "Single elimination",
                Topology::Double => "Double elimination",
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
                    // TODO: i18n once copy is approved.
                    <p>"A match becomes available as soon as both players are known."</p>
                    {(configuration.topology == Topology::Double)
                        .then(|| {
                            view! {
                                // TODO: i18n once copy is approved.
                                <p>
                                    "If the lower-bracket winner wins the final, a second final determines the champion. Each final appears separately in the bracket."
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

pub(crate) fn round_robin_criterion_name(
    i18n: I18nContext<Locale, I18nKeys>,
    configuration: &RoundRobinConfig,
    criterion: RoundRobinCriterion,
) -> String {
    if criterion == RoundRobinCriterion::PrimaryScore {
        return match configuration.primary_score {
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
        match configuration.primary_score {
            RoundRobinPrimaryScore::GamePoints => RoundRobinCriterion::GamePoints,
            RoundRobinPrimaryScore::MatchPoints => RoundRobinCriterion::MatchPoints,
        }
    } else {
        criterion
    };
    // TODO: i18n once copy is approved.
    if criterion == RoundRobinCriterion::MatchPoints {
        return format!("All {} games against each opponent form one match. Compare the total game points to determine the match win, draw, or loss. Match points are awarded only after every game has a result, even if the match is already clinched. Partial and split forfeits count in the aggregate; if every game is a double forfeit, both players receive zero match points. Resting rounds award no points.", configuration.repeats.get());
    }
    let explanation = criterion.explanation(i18n);
    if matches!(
        criterion,
        RoundRobinCriterion::DirectEncounter(_)
            | RoundRobinCriterion::SonnebornBerger(_)
            | RoundRobinCriterion::Koya(_)
            | RoundRobinCriterion::Progressive(_)
    ) {
        let basis = match configuration.primary_score {
            RoundRobinPrimaryScore::GamePoints => "game points",
            RoundRobinPrimaryScore::MatchPoints => "match points",
        };
        // TODO: i18n once copy is approved.
        return format!("{explanation} Uses {basis}, the tournament's primary score.");
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
                            .standings
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
                    // TODO: i18n once copy is approved.
                    <h3 class="text-sm font-bold">"Scoring"</h3>
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
                                // TODO: i18n once copy is approved.
                                <h3 class="text-sm font-bold">"Tiebreaks"</h3>
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
