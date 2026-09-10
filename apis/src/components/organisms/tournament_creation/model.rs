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
use tournamint::{elimination::topology as build_elimination_topology, PlayerId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PlanLocation {
    Default,
    Override(usize),
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

    pub(super) fn normalize_time_mode(&mut self, replacement: Clock) {
        for phase in &mut self.phases {
            if phase.clock.mode() != replacement.mode() {
                phase.clock = replacement;
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EliminationOverrideDraft {
    pub stage: Stage,
    pub plan: EliminationPlanDraft,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EliminationCreationDraft {
    pub(super) advanced: bool,
    advanced_initialized: bool,
    pub(super) default_plan: EliminationPlanDraft,
    pub(super) overrides: Vec<EliminationOverrideDraft>,
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
            advanced: false,
            advanced_initialized: false,
            default_plan: EliminationPlanDraft::balanced(clock),
            overrides: Vec::new(),
        }
    }

    pub(super) fn use_simple_preset(&mut self) {
        self.advanced = false;
    }

    pub(super) fn use_advanced_editor(&mut self, clock: Clock) {
        if !self.advanced_initialized {
            self.default_plan = EliminationPlanDraft::balanced(clock);
            self.advanced_initialized = true;
        }
        self.advanced = true;
    }

    pub(crate) fn normalize_time_mode(&mut self, replacement: Clock) {
        self.default_plan.normalize_time_mode(replacement);
        for override_ in &mut self.overrides {
            override_.plan.normalize_time_mode(replacement);
        }
    }

    pub(super) fn invalid_override_stages(&self, available_stages: &[Stage]) -> Vec<Stage> {
        let mut seen = Vec::new();
        let mut invalid = Vec::new();
        for override_ in &self.overrides {
            if (!available_stages.contains(&override_.stage) || seen.contains(&override_.stage))
                && !invalid.contains(&override_.stage)
            {
                invalid.push(override_.stage);
            }
            seen.push(override_.stage);
        }
        invalid
    }

    pub(super) fn plan(&self, location: PlanLocation) -> Option<&EliminationPlanDraft> {
        match location {
            PlanLocation::Default => Some(&self.default_plan),
            PlanLocation::Override(index) => self.overrides.get(index).map(|value| &value.plan),
        }
    }

    pub(super) fn plan_mut(&mut self, location: PlanLocation) -> Option<&mut EliminationPlanDraft> {
        match location {
            PlanLocation::Default => Some(&mut self.default_plan),
            PlanLocation::Override(index) => {
                self.overrides.get_mut(index).map(|value| &mut value.plan)
            }
        }
    }

    pub(super) fn add_override(&mut self, available_stages: &[Stage], stage: Stage) {
        if !available_override_stages(available_stages, &self.overrides).contains(&stage) {
            return;
        }
        self.overrides.push(EliminationOverrideDraft {
            stage,
            plan: self.default_plan.clone(),
        });
    }

    pub(super) fn effective_default_plan(
        &self,
        tournament_clock: Clock,
    ) -> Result<SeriesPlan, EliminationCreationError> {
        if self.advanced {
            self.default_plan.to_plan(tournament_clock)
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
    InvalidStageOverrides(Vec<Stage>),
    PreviewOrdinalOverflow,
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
            Self::InvalidStageOverrides(stages) => {
                write!(formatter, "These stage overrides no longer exist in the selected bracket: {stages:?}")
            }
            Self::PreviewOrdinalOverflow => {
                formatter.write_str("This plan is too large to preview safely.")
            }
        }
    }
}

pub(crate) fn build_elimination_creation_configuration(
    draft: &EliminationCreationDraft,
    topology: Topology,
    available_stages: &[Stage],
    tournament_clock: Clock,
) -> Result<Config, EliminationCreationError> {
    if draft.advanced {
        let invalid = draft.invalid_override_stages(available_stages);
        if !invalid.is_empty() {
            return Err(EliminationCreationError::InvalidStageOverrides(invalid));
        }
    }
    let default_plan = draft.effective_default_plan(tournament_clock)?;
    let stage_overrides = if draft.advanced {
        draft
            .overrides
            .iter()
            .map(|override_| {
                Ok(StageOverride {
                    stage: override_.stage,
                    plan: override_.plan.to_plan(tournament_clock)?,
                })
            })
            .collect::<Result<Vec<_>, EliminationCreationError>>()?
    } else {
        Vec::new()
    };
    Ok(Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Elimination(EliminationConfig {
            topology,
            default_plan,
            stage_overrides,
        }),
    })
}

pub(super) fn available_override_stages(
    available_stages: &[Stage],
    overrides: &[EliminationOverrideDraft],
) -> Vec<Stage> {
    available_stages
        .iter()
        .copied()
        .filter(|stage| !overrides.iter().any(|override_| override_.stage == *stage))
        .collect()
}

pub(crate) fn elimination_stages_for_field(
    topology: Topology,
    participant_count: usize,
) -> Vec<Stage> {
    let seeds = (0..participant_count.max(2))
        .map(PlayerId::new)
        .collect::<Vec<_>>();
    let Ok(definition) = build_elimination_topology(topology, &seeds) else {
        return Vec::new();
    };
    let mut stages = Vec::new();
    for node in definition.nodes() {
        if !stages.contains(&node.stage) {
            stages.push(node.stage);
        }
    }
    stages
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum PreviewItem {
    Phase {
        phase_ordinal: u16,
        games_per_set: u16,
        set_limit: SetLimit,
        clinch: ClinchPolicy,
        clock: Clock,
    },
    Game {
        phase_ordinal: u16,
        set_ordinal: u16,
        game_ordinal_in_set: u16,
        game_ordinal_in_series: u32,
        white: EntrantSide,
    },
    RepeatedFiniteSets {
        phase_ordinal: u16,
        additional_sets: u16,
    },
    TieTransition {
        from_phase: u16,
        to_phase: u16,
    },
    RepeatUntilDecisive {
        phase_ordinal: u16,
    },
}

pub(super) fn preview_plan(
    plan: &SeriesPlan,
) -> Result<Vec<PreviewItem>, EliminationCreationError> {
    const MAXIMUM_PREVIEW_GAMES: usize = MAX_PHASES * MAX_GAMES_PER_SET as usize;
    let mut items = Vec::new();
    let mut game_index = 0_usize;
    for (phase_index, phase) in plan.phases.iter().enumerate() {
        let phase_ordinal = u16::try_from(phase_index + 1)
            .map_err(|_| EliminationCreationError::PreviewOrdinalOverflow)?;
        items.push(PreviewItem::Phase {
            phase_ordinal,
            games_per_set: phase.games_per_set,
            set_limit: phase.set_limit,
            clinch: phase.clinch,
            clock: phase.clock,
        });
        if phase.color_order.len() != usize::from(phase.games_per_set) {
            return Err(EliminationCreationError::ColorOrderLengthMismatch { phase: phase_index });
        }
        let phase_start = game_index;
        for game_in_set in 0..phase.games_per_set {
            if items.len() >= MAXIMUM_PREVIEW_GAMES + plan.phases.len() * 2 {
                return Err(EliminationCreationError::PreviewOrdinalOverflow);
            }
            let white = phase.color_order[usize::from(game_in_set)];
            items.push(PreviewItem::Game {
                phase_ordinal,
                set_ordinal: 1,
                game_ordinal_in_set: game_in_set + 1,
                game_ordinal_in_series: u32::try_from(phase_start + usize::from(game_in_set) + 1)
                    .map_err(|_| {
                    EliminationCreationError::PreviewOrdinalOverflow
                })?,
                white,
            });
        }
        let represented_sets = match phase.set_limit {
            SetLimit::AtMost(limit) => {
                if limit.get() > 1 {
                    items.push(PreviewItem::RepeatedFiniteSets {
                        phase_ordinal,
                        additional_sets: limit.get() - 1,
                    });
                }
                usize::from(limit.get())
            }
            SetLimit::UntilDecisive => {
                items.push(PreviewItem::RepeatUntilDecisive { phase_ordinal });
                1
            }
        };
        game_index = game_index
            .checked_add(
                usize::from(phase.games_per_set)
                    .checked_mul(represented_sets)
                    .ok_or(EliminationCreationError::PreviewOrdinalOverflow)?,
            )
            .ok_or(EliminationCreationError::PreviewOrdinalOverflow)?;
        if u32::try_from(game_index).is_err() {
            return Err(EliminationCreationError::PreviewOrdinalOverflow);
        }
        if phase_index + 1 < plan.phases.len() {
            items.push(PreviewItem::TieTransition {
                from_phase: phase_ordinal,
                to_phase: phase_ordinal + 1,
            });
        }
    }
    Ok(items)
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
    fn advanced_plan_serializes_odd_sets_phases_and_exact_override() {
        let slow = realtime(10, 5);
        let edited_tournament_clock = realtime(15, 10);
        let fast = realtime(3, 2);
        let mut draft = EliminationCreationDraft::new(slow);
        draft.advanced = true;
        draft.default_plan = EliminationPlanDraft {
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
        draft.overrides.push(EliminationOverrideDraft {
            stage: Stage::SingleFinal,
            plan: EliminationPlanDraft::balanced(fast),
        });

        let configuration = build_elimination_creation_configuration(
            &draft,
            Topology::Single { bronze: true },
            &elimination_stages_for_field(Topology::Single { bronze: true }, 4),
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
    fn advanced_plan_rejects_cross_mode_phase_and_incompatible_stage() {
        let realtime = realtime(5, 3);
        let correspondence = Clock::Correspondence(CorrespondenceClock::DaysPerMove {
            seconds_per_move: NonZeroU32::new(86_400).unwrap(),
        });
        let mut draft = EliminationCreationDraft::new(realtime);
        draft.advanced = true;
        draft.default_plan.phases[0].use_tournament_clock = false;
        draft.default_plan.phases[0].clock = correspondence;
        assert_eq!(
            build_elimination_creation_configuration(
                &draft,
                Topology::Single { bronze: false },
                &elimination_stages_for_field(Topology::Single { bronze: false }, 4),
                realtime,
            ),
            Err(EliminationCreationError::MixedTimeModes),
        );

        draft.default_plan.phases[0].clock = realtime;
        draft.overrides.push(EliminationOverrideDraft {
            stage: Stage::Bronze,
            plan: EliminationPlanDraft::balanced(realtime),
        });
        assert!(matches!(
            build_elimination_creation_configuration(
                &draft,
                Topology::Double,
                &elimination_stages_for_field(Topology::Double, 4),
                realtime,
            ),
            Err(EliminationCreationError::InvalidStageOverrides(stages))
                if stages == vec![Stage::Bronze]
        ));
    }

    #[test]
    fn advanced_plan_validation_rejects_invalid_inventory_and_zero_sizes() {
        let clock = realtime(5, 3);
        let mut draft = EliminationCreationDraft::new(clock);
        draft.advanced = true;
        draft.default_plan.phases[0].color_order = vec![EntrantSide::First, EntrantSide::First];
        assert_eq!(
            build_elimination_creation_configuration(
                &draft,
                Topology::Single { bronze: false },
                &elimination_stages_for_field(Topology::Single { bronze: false }, 4),
                clock,
            ),
            Err(EliminationCreationError::InvalidColorInventory { phase: 0 }),
        );

        draft.default_plan.phases[0].color_order.clear();
        draft.default_plan.phases[0].games_per_set = 0;
        assert_eq!(
            build_elimination_creation_configuration(
                &draft,
                Topology::Single { bronze: false },
                &elimination_stages_for_field(Topology::Single { bronze: false }, 4),
                clock,
            ),
            Err(EliminationCreationError::GamesPerSetMustBePositive),
        );
    }

    #[test]
    fn field_change_preserves_and_reports_invalid_stage_overrides() {
        let clock = realtime(5, 3);
        let mut draft = EliminationCreationDraft::new(clock);
        draft.advanced = true;
        draft.overrides = vec![
            EliminationOverrideDraft {
                stage: Stage::SingleFinal,
                plan: EliminationPlanDraft::balanced(clock),
            },
            EliminationOverrideDraft {
                stage: Stage::SingleFinal,
                plan: EliminationPlanDraft::balanced(clock),
            },
            EliminationOverrideDraft {
                stage: Stage::Bronze,
                plan: EliminationPlanDraft::balanced(clock),
            },
            EliminationOverrideDraft {
                stage: Stage::GrandFinal,
                plan: EliminationPlanDraft::balanced(clock),
            },
        ];

        let invalid = draft.invalid_override_stages(&elimination_stages_for_field(
            Topology::Single { bronze: false },
            4,
        ));
        assert_eq!(
            invalid,
            vec![Stage::SingleFinal, Stage::Bronze, Stage::GrandFinal]
        );
        assert_eq!(
            draft.overrides.len(),
            4,
            "invalid overrides remain editable"
        );
    }
}
