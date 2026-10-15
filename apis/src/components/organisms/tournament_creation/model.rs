use shared_types::tournament::{
    elimination::{
        ClinchPolicy,
        Config as EliminationConfig,
        EntrantSide,
        SeriesPhase,
        SeriesPlan,
        SetLimit,
        Stage,
        StageOverride,
        Topology,
        MAX_FINITE_SETS,
        MAX_GAMES_PER_SET,
        MAX_PHASES,
    },
    BotAdmission,
    Clock,
    Config,
    FormatConfig,
    RealtimeClock,
};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    num::{NonZeroU16, NonZeroU32},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PlanLocation {
    Custom,
    Finals,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EliminationPhaseDraft {
    pub games_per_set: u16,
    pub color_order: Vec<EntrantSide>,
    /// Used whenever this phase is not last. The last phase is always
    /// `UntilDecisive`, which keeps unreachable plan shapes out of the form.
    pub finite_sets: u16,
    pub clinch: ClinchPolicy,
    pub use_tournament_clock: bool,
    pub clock: Clock,
}

impl EliminationPhaseDraft {
    pub(super) fn balanced(clock: Clock) -> Self {
        Self {
            games_per_set: 2,
            color_order: vec![EntrantSide::First, EntrantSide::Second],
            finite_sets: 1,
            clinch: ClinchPolicy::PlayAll,
            use_tournament_clock: true,
            clock,
        }
    }

    pub(super) fn set_games_per_set(&mut self, games_per_set: u16) {
        let advantaged = self.odd_color_advantage().unwrap_or(EntrantSide::First);
        self.games_per_set = games_per_set;
        self.color_order = alternating_color_order(games_per_set, advantaged);
    }

    pub(super) fn odd_color_advantage(&self) -> Option<EntrantSide> {
        if self.games_per_set.is_multiple_of(2)
            || self.color_order.len() != usize::from(self.games_per_set)
        {
            return None;
        }
        let first = self
            .color_order
            .iter()
            .filter(|side| **side == EntrantSide::First)
            .count();
        let second = self.color_order.len() - first;
        match first.cmp(&second) {
            std::cmp::Ordering::Greater => Some(EntrantSide::First),
            std::cmp::Ordering::Less => Some(EntrantSide::Second),
            std::cmp::Ordering::Equal => None,
        }
    }

    pub(super) fn give_odd_color_advantage_to(&mut self, side: EntrantSide) {
        let Some(current) = self.odd_color_advantage() else {
            return;
        };
        if current != side {
            for color in &mut self.color_order {
                *color = opposite(*color);
            }
        }
    }
}

fn alternating_color_order(games_per_set: u16, first: EntrantSide) -> Vec<EntrantSide> {
    (0..games_per_set)
        .map(|index| {
            if index % 2 == 0 {
                first
            } else {
                opposite(first)
            }
        })
        .collect()
}

const fn opposite(side: EntrantSide) -> EntrantSide {
    match side {
        EntrantSide::First => EntrantSide::Second,
        EntrantSide::Second => EntrantSide::First,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EliminationPlanDraft {
    pub phases: Vec<EliminationPhaseDraft>,
}

impl EliminationPlanDraft {
    fn balanced(clock: Clock) -> Self {
        Self {
            phases: vec![EliminationPhaseDraft::balanced(clock)],
        }
    }

    pub(super) fn to_plan(
        &self,
        tournament_clock: Clock,
    ) -> Result<SeriesPlan, EliminationCreationError> {
        if self.phases.is_empty() {
            return Err(EliminationCreationError::EmptyPhaseList);
        }
        let last_phase = self.phases.len() - 1;
        let phases = self
            .phases
            .iter()
            .enumerate()
            .map(|(index, phase)| {
                let clock = if phase.use_tournament_clock {
                    tournament_clock
                } else {
                    phase.clock
                };
                if clock.mode() != tournament_clock.mode() {
                    return Err(EliminationCreationError::MixedTimeModes);
                }
                if phase.games_per_set == 0 {
                    return Err(EliminationCreationError::GamesPerSetMustBePositive);
                }
                if phase.color_order.len() != usize::from(phase.games_per_set) {
                    return Err(EliminationCreationError::ColorOrderLengthMismatch {
                        phase: index,
                    });
                }
                let first = phase
                    .color_order
                    .iter()
                    .filter(|side| **side == EntrantSide::First)
                    .count();
                let second = phase.color_order.len() - first;
                if first.abs_diff(second) > 1 {
                    return Err(EliminationCreationError::InvalidColorInventory { phase: index });
                }
                let set_limit = if index == last_phase {
                    SetLimit::UntilDecisive
                } else {
                    SetLimit::AtMost(
                        NonZeroU16::new(phase.finite_sets)
                            .ok_or(EliminationCreationError::FiniteSetLimitMustBePositive)?,
                    )
                };
                Ok(SeriesPhase {
                    games_per_set: phase.games_per_set,
                    color_order: phase.color_order.clone(),
                    set_limit,
                    clinch: phase.clinch,
                    clock,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SeriesPlan { phases })
    }

    pub(super) fn extend_if_tied(&mut self) -> Option<usize> {
        if self.phases.len() >= EDITOR_MAX_PHASES {
            return None;
        }
        let mut next = self.phases.last()?.clone();
        next.finite_sets = 1;
        self.phases.push(next);
        Some(self.phases.len() - 1)
    }

    pub(super) fn normalize_time_mode(&mut self, replacement: Clock) {
        for phase in &mut self.phases {
            if phase.clock.mode() != replacement.mode() {
                phase.clock = replacement;
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EliminationCreationDraft {
    pub(super) custom: bool,
    pub(super) custom_plan: EliminationPlanDraft,
    pub(super) finals: Option<EliminationPlanDraft>,
}

impl Default for EliminationCreationDraft {
    fn default() -> Self {
        let clock = Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(600).expect("fixed clock is positive"),
            increment_seconds: 10,
        });
        Self::new(clock)
    }
}

impl EliminationCreationDraft {
    pub(crate) fn new(clock: Clock) -> Self {
        Self {
            custom: false,
            custom_plan: EliminationPlanDraft::balanced(clock),
            finals: None,
        }
    }

    pub(super) fn customize_final(&mut self, clock: Clock) {
        self.finals = Some(if self.custom {
            self.custom_plan.clone()
        } else {
            EliminationPlanDraft::balanced(clock)
        });
    }

    pub(crate) fn normalize_time_mode(&mut self, replacement: Clock) {
        self.custom_plan.normalize_time_mode(replacement);
        if let Some(finals) = &mut self.finals {
            finals.normalize_time_mode(replacement);
        }
    }

    pub(super) fn plan(&self, location: PlanLocation) -> Option<&EliminationPlanDraft> {
        match location {
            PlanLocation::Custom => Some(&self.custom_plan),
            PlanLocation::Finals => self.finals.as_ref(),
        }
    }

    pub(super) fn plan_mut(&mut self, location: PlanLocation) -> Option<&mut EliminationPlanDraft> {
        match location {
            PlanLocation::Custom => Some(&mut self.custom_plan),
            PlanLocation::Finals => self.finals.as_mut(),
        }
    }

    pub(super) fn effective_default_plan(
        &self,
        tournament_clock: Clock,
    ) -> Result<SeriesPlan, EliminationCreationError> {
        if self.custom {
            self.custom_plan.to_plan(tournament_clock)
        } else {
            Ok(SeriesPlan::balanced(tournament_clock))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EliminationCreationError {
    EmptyPhaseList,
    GamesPerSetMustBePositive,
    ColorOrderLengthMismatch { phase: usize },
    InvalidColorInventory { phase: usize },
    FiniteSetLimitMustBePositive,
    MixedTimeModes,
}

impl Display for EliminationCreationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::EmptyPhaseList => formatter.write_str("Add at least one phase."),
            Self::GamesPerSetMustBePositive => {
                formatter.write_str("Games per set must be positive.")
            }
            Self::ColorOrderLengthMismatch { phase } => write!(
                formatter,
                "Phase {} needs exactly one color assignment per game.",
                phase + 1
            ),
            Self::InvalidColorInventory { phase } => write!(
                formatter,
                "Phase {} must split colors equally, apart from one unavoidable extra assignment in an odd set.",
                phase + 1
            ),
            Self::FiniteSetLimitMustBePositive => {
                formatter.write_str("Finite phases must allow at least one set.")
            }
            Self::MixedTimeModes => {
                formatter.write_str("Every elimination phase must use the tournament's time mode.")
            }
        }
    }
}

pub(crate) fn build_elimination_creation_configuration(
    draft: &EliminationCreationDraft,
    topology: Topology,
    tournament_clock: Clock,
) -> Result<Config, EliminationCreationError> {
    let default_plan = draft.effective_default_plan(tournament_clock)?;
    let stage_overrides = if let Some(finals) = &draft.finals {
        let plan = finals.to_plan(tournament_clock)?;
        let stages = match topology {
            Topology::Single { bronze: false } => vec![Stage::SingleFinal],
            Topology::Single { bronze: true } => vec![Stage::SingleFinal, Stage::Bronze],
            Topology::Double => vec![Stage::GrandFinal, Stage::Reset],
        };
        stages
            .into_iter()
            .map(|stage| StageOverride {
                stage,
                plan: plan.clone(),
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok(Config {
        bot_admission: BotAdmission::HumansOnly,
        format: FormatConfig::Elimination(EliminationConfig {
            topology,
            default_plan,
            stage_overrides,
        }),
    })
}

pub(super) const EDITOR_MAX_PHASES: usize = MAX_PHASES;
pub(super) const EDITOR_MAX_GAMES_PER_SET: u16 = MAX_GAMES_PER_SET;
pub(super) const EDITOR_MAX_FINITE_SETS: u16 = MAX_FINITE_SETS;

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::tournament::CorrespondenceClock;

    fn realtime(minutes: u32, increment_seconds: u32) -> Clock {
        Clock::Realtime(RealtimeClock {
            base_seconds: NonZeroU32::new(minutes * 60).unwrap(),
            increment_seconds,
        })
    }

    #[test]
    fn custom_plan_serializes_odd_sets_phases_and_finals() {
        let slow = realtime(10, 5);
        let edited_tournament_clock = realtime(15, 10);
        let fast = realtime(3, 2);
        let mut draft = EliminationCreationDraft::new(slow);
        draft.custom = true;
        draft.custom_plan = EliminationPlanDraft {
            phases: vec![
                EliminationPhaseDraft {
                    games_per_set: 3,
                    color_order: vec![EntrantSide::First, EntrantSide::Second, EntrantSide::First],
                    finite_sets: 2,
                    clinch: ClinchPolicy::EarlyClinch,
                    use_tournament_clock: true,
                    clock: slow,
                },
                EliminationPhaseDraft {
                    games_per_set: 1,
                    color_order: vec![EntrantSide::Second],
                    finite_sets: 31,
                    clinch: ClinchPolicy::PlayAll,
                    use_tournament_clock: false,
                    clock: fast,
                },
            ],
        };
        draft.finals = Some(EliminationPlanDraft::balanced(fast));

        let configuration = build_elimination_creation_configuration(
            &draft,
            Topology::Single { bronze: true },
            edited_tournament_clock,
        )
        .unwrap();
        let encoded = serde_json::to_string(&configuration).unwrap();
        let decoded: Config = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, configuration);
        let FormatConfig::Elimination(elimination) = configuration.format else {
            panic!("expected elimination configuration");
        };
        assert_eq!(
            elimination.default_plan.phases[0].set_limit,
            SetLimit::AtMost(NonZeroU16::new(2).unwrap())
        );
        assert_eq!(
            elimination.default_plan.phases[1].set_limit,
            SetLimit::UntilDecisive
        );
        assert_eq!(
            elimination.default_plan.phases[0].clock, edited_tournament_clock,
            "tournament-clock steps follow later primary-clock edits"
        );
        assert_eq!(elimination.default_plan.phases[1].clock, fast);
        assert_eq!(elimination.stage_overrides[0].stage, Stage::SingleFinal);
    }

    #[test]
    fn custom_plan_rejects_cross_mode_phase() {
        let realtime = realtime(5, 3);
        let correspondence = Clock::Correspondence(CorrespondenceClock::DaysPerMove {
            seconds_per_move: NonZeroU32::new(86_400).unwrap(),
        });
        let mut draft = EliminationCreationDraft::new(realtime);
        draft.custom = true;
        draft.custom_plan.phases[0].use_tournament_clock = false;
        draft.custom_plan.phases[0].clock = correspondence;
        assert_eq!(
            build_elimination_creation_configuration(
                &draft,
                Topology::Single { bronze: false },
                realtime,
            ),
            Err(EliminationCreationError::MixedTimeModes),
        );
    }

    #[test]
    fn custom_plan_validation_rejects_invalid_inventory_and_zero_sizes() {
        let clock = realtime(5, 3);
        let mut draft = EliminationCreationDraft::new(clock);
        draft.custom = true;
        draft.custom_plan.phases[0].color_order = vec![EntrantSide::First, EntrantSide::First];
        assert_eq!(
            build_elimination_creation_configuration(
                &draft,
                Topology::Single { bronze: false },
                clock,
            ),
            Err(EliminationCreationError::InvalidColorInventory { phase: 0 }),
        );

        draft.custom_plan.phases[0].color_order.clear();
        draft.custom_plan.phases[0].games_per_set = 0;
        assert_eq!(
            build_elimination_creation_configuration(
                &draft,
                Topology::Single { bronze: false },
                clock,
            ),
            Err(EliminationCreationError::GamesPerSetMustBePositive),
        );
    }

    #[test]
    fn builder_maps_finals_to_every_applicable_stage_for_both_base_formats() {
        let clock = realtime(5, 3);
        for custom in [false, true] {
            let mut draft = EliminationCreationDraft::new(clock);
            draft.custom = custom;
            draft.custom_plan.phases[0].set_games_per_set(3);
            let expected_base = if custom {
                draft.custom_plan.to_plan(clock).unwrap()
            } else {
                SeriesPlan::balanced(clock)
            };
            for (topology, stages) in [
                (Topology::Single { bronze: false }, vec![Stage::SingleFinal]),
                (
                    Topology::Single { bronze: true },
                    vec![Stage::SingleFinal, Stage::Bronze],
                ),
                (Topology::Double, vec![Stage::GrandFinal, Stage::Reset]),
            ] {
                draft.finals = None;
                let inherited =
                    build_elimination_creation_configuration(&draft, topology, clock).unwrap();
                let FormatConfig::Elimination(inherited) = inherited.format else {
                    panic!("expected elimination configuration");
                };
                assert_eq!(inherited.default_plan, expected_base);
                assert!(inherited.stage_overrides.is_empty());

                draft.customize_final(clock);
                let finals = draft.finals.as_mut().unwrap();
                assert_eq!(finals.to_plan(clock).unwrap(), expected_base);
                finals.phases[0].set_games_per_set(4);
                finals.phases[0].use_tournament_clock = false;
                finals.phases[0].clock = realtime(3, 2);
                let expected_final = finals.to_plan(clock).unwrap();
                let configuration =
                    build_elimination_creation_configuration(&draft, topology, clock).unwrap();
                let FormatConfig::Elimination(elimination) = configuration.format else {
                    panic!("expected elimination configuration");
                };
                assert_eq!(elimination.topology, topology);
                assert_eq!(elimination.default_plan, expected_base);
                assert_eq!(
                    elimination.stage_overrides,
                    stages
                        .into_iter()
                        .map(|stage| {
                            StageOverride {
                                stage,
                                plan: expected_final.clone(),
                            }
                        })
                        .collect::<Vec<_>>()
                );
            }
        }
    }

    #[test]
    fn balanced_ignores_inactive_custom_but_rejects_invalid_enabled_finals() {
        let clock = realtime(5, 3);
        let topology = Topology::Single { bronze: false };
        let mut draft = EliminationCreationDraft::new(clock);
        draft.custom_plan.phases[0].set_games_per_set(0);
        let configuration =
            build_elimination_creation_configuration(&draft, topology, clock).unwrap();
        let FormatConfig::Elimination(elimination) = configuration.format else {
            panic!("expected elimination configuration");
        };
        assert_eq!(elimination.default_plan, SeriesPlan::balanced(clock));
        draft.customize_final(clock);
        draft.finals.as_mut().unwrap().phases[0].set_games_per_set(0);
        assert_eq!(
            build_elimination_creation_configuration(&draft, topology, clock),
            Err(EliminationCreationError::GamesPerSetMustBePositive),
        );
        draft.finals = None;
        draft.custom = true;
        assert_eq!(
            build_elimination_creation_configuration(&draft, topology, clock),
            Err(EliminationCreationError::GamesPerSetMustBePositive),
        );
    }

    #[test]
    fn extending_ties_preserves_settings_and_finite_limit_with_only_last_until_decisive() {
        let original_clock = realtime(10, 5);
        let edited_clock = realtime(15, 10);
        for inherited in [false, true] {
            let mut plan = EliminationPlanDraft::balanced(original_clock);
            let phase = &mut plan.phases[0];
            phase.set_games_per_set(3);
            phase.color_order = vec![EntrantSide::Second, EntrantSide::First, EntrantSide::First];
            phase.finite_sets = 3;
            phase.clinch = ClinchPolicy::EarlyClinch;
            phase.use_tournament_clock = inherited;
            let original = phase.clone();
            assert_eq!(plan.extend_if_tied(), Some(1));
            assert_eq!(plan.phases[0], original);
            assert_eq!(
                plan.phases[1],
                EliminationPhaseDraft {
                    finite_sets: 1,
                    ..original
                }
            );
            let built = plan.to_plan(edited_clock).unwrap();
            assert_eq!(
                built.phases[0].set_limit,
                SetLimit::AtMost(NonZeroU16::new(3).unwrap())
            );
            assert_eq!(built.phases[1].set_limit, SetLimit::UntilDecisive);
            for phase in built.phases {
                assert_eq!(
                    phase.clock,
                    if inherited {
                        edited_clock
                    } else {
                        original_clock
                    }
                );
            }
            while plan.phases.len() < EDITOR_MAX_PHASES {
                assert!(plan.extend_if_tied().is_some());
            }
            let full = plan.clone();
            assert_eq!(plan.extend_if_tied(), None);
            assert_eq!(plan, full);
        }
    }

    #[test]
    fn time_mode_changes_normalize_preserved_custom_and_finals_clocks() {
        let clock = realtime(5, 3);
        let correspondence = Clock::Correspondence(CorrespondenceClock::DaysPerMove {
            seconds_per_move: NonZeroU32::new(86_400).unwrap(),
        });
        let mut draft = EliminationCreationDraft::new(clock);
        draft.custom_plan.phases[0].use_tournament_clock = false;
        draft.customize_final(clock);
        draft.finals.as_mut().unwrap().phases[0].use_tournament_clock = false;
        draft.normalize_time_mode(correspondence);
        for plan in [&draft.custom_plan, draft.finals.as_ref().unwrap()] {
            assert_eq!(
                plan.to_plan(correspondence).unwrap().phases[0].clock,
                correspondence
            );
            assert!(!plan.phases[0].use_tournament_clock);
        }
    }
}
