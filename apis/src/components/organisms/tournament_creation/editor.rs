use super::model::{
    available_override_stages,
    preview_plan,
    EliminationCreationDraft,
    EliminationCreationError,
    EliminationPhaseDraft,
    EliminationPlanDraft,
    PlanLocation,
    PreviewItem,
    EDITOR_MAX_FINITE_SETS,
    EDITOR_MAX_GAMES_PER_SET,
    EDITOR_MAX_PHASES,
};
use crate::{
    components::{
        atoms::simple_switch::SimpleSwitch,
        molecules::{panel::Panel, time_row::format_compact_duration},
    },
    i18n::*,
};
use leptos::prelude::*;
use leptos_i18n::I18nContext;
use shared_types::tournament::{
    elimination::{ClinchPolicy, EntrantSide, SeriesPlan, SetLimit, Stage, Topology},
    Clock,
    CorrespondenceClock,
};
use std::num::NonZeroU32;

const SECONDS_PER_DAY: u32 = 86_400;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct OverrideStageOptions {
    invalid: Vec<Stage>,
    non_final: Vec<Stage>,
    final_: Vec<Stage>,
}

#[component]
pub(crate) fn EliminationCreationEditor(
    draft: RwSignal<EliminationCreationDraft>,
    topology: Signal<Topology>,
    stages: Memo<Vec<Stage>>,
    tournament_clock: Signal<Clock>,
    bronze_match: RwSignal<bool>,
    reveal_invalid: RwSignal<u32>,
    children: Children,
) -> impl IntoView {
    let i18n = use_i18n();
    let expanded = RwSignal::new(Some((PlanLocation::Default, 0_usize)));
    let reveal_focus = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        if reveal_invalid.get() == 0 {
            return;
        }
        let clock = tournament_clock.get_untracked();
        let invalid = draft.with_untracked(|draft| {
            std::iter::once(PlanLocation::Default)
                .chain((0..draft.overrides.len()).map(PlanLocation::Override))
                .find_map(|location| {
                    draft.plan(location).and_then(|plan| {
                        plan.phases
                            .iter()
                            .enumerate()
                            .find(|(index, phase)| {
                                phase_error(phase, *index + 1 == plan.phases.len(), clock).is_some()
                            })
                            .map(|(index, _)| (location, index))
                    })
                })
        });
        expanded.set(invalid.or(Some((PlanLocation::Default, 0))));
        if let Some(element) = reveal_focus.get_untracked() {
            element.scroll_into_view();
            let _ = element.focus();
        }
    });
    let override_chooser_open = RwSignal::new(false);
    let final_override_chooser_open = RwSignal::new(false);
    let override_stage_options = Memo::new(move |_| {
        stages.with(|stages| {
            draft.with(|draft| {
                let invalid = draft.invalid_override_stages(stages);
                let (final_, non_final) = available_override_stages(stages, &draft.overrides)
                    .into_iter()
                    .partition(|stage| is_final_stage(*stage));
                OverrideStageOptions {
                    invalid,
                    non_final,
                    final_,
                }
            })
        })
    });
    let invalid_stages =
        Signal::derive(move || override_stage_options.with(|options| options.invalid.clone()));
    let available_non_final_stages =
        Signal::derive(move || override_stage_options.with(|options| options.non_final.clone()));
    let available_final_stages =
        Signal::derive(move || override_stage_options.with(|options| options.final_.clone()));
    let simple_preview = Signal::derive(move || Ok(SeriesPlan::balanced(tournament_clock.get())));

    view! {
        <div node_ref=reveal_focus tabindex="-1">
            <Panel
                // TODO: i18n once copy is approved.
                title="Match format"
                body_class="space-y-5"
            >
                {children()}
                <div class="flex flex-wrap gap-2">
                    <button
                        type="button"
                        class=move || {
                            if !draft.with(|draft| draft.advanced) {
                                "ui-button ui-button-primary ui-button-sm"
                            } else {
                                "ui-button ui-button-secondary ui-button-sm"
                            }
                        }
                        on:click=move |_| {
                            draft.update(EliminationCreationDraft::use_simple_preset);
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Balanced"
                    </button>
                    <button
                        type="button"
                        class=move || {
                            if draft.with(|draft| draft.advanced) {
                                "ui-button ui-button-primary ui-button-sm"
                            } else {
                                "ui-button ui-button-secondary ui-button-sm"
                            }
                        }
                        on:click=move |_| {
                            draft
                                .update(|draft| {
                                    draft.use_advanced_editor(tournament_clock.get_untracked())
                                });
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Custom"
                    </button>
                </div>

                <Show
                    when=move || draft.with(|draft| draft.advanced)
                    fallback=move || {
                        view! {
                            <div class="space-y-3">
                                <p class="ui-notice">
                                    // TODO: i18n once copy is approved.
                                    "Play two games, one as each color. If tied, repeat with colors reversed."
                                </p>
                                <SeriesPreview plan=simple_preview />
                            </div>
                        }
                    }
                >
                    <div class="space-y-5">
                        <p class="ui-notice">
                            // TODO: i18n once copy is approved.
                            "Choose the opening games and the extra games played if tied."
                        </p>
                        <SeriesPlanEditor
                            draft
                            location=PlanLocation::Default
                            tournament_clock
                            expanded
                        />

                        <details
                            class="space-y-3"
                            prop:open=move || {
                                matches!(expanded.get(), Some((PlanLocation::Override(_), _)))
                                    || !invalid_stages.with(Vec::is_empty)
                            }
                        >
                            // TODO: i18n once copy is approved.
                            <summary class="font-semibold cursor-pointer">
                                {move || {
                                    format!(
                                        "Different formats by round ({})",
                                        draft.with(|draft| draft.overrides.len()),
                                    )
                                }}
                            </summary>
                            <div class="flex flex-wrap gap-3 justify-between items-center">
                                <div>
                                    // TODO: i18n once copy is approved.
                                    <p class="ui-field-helper">
                                        "Use a different match format for a particular round."
                                    </p>
                                </div>
                                <button
                                    type="button"
                                    class="ui-button ui-button-secondary ui-button-sm"
                                    prop:disabled=move || {
                                        available_non_final_stages.with(Vec::is_empty)
                                    }
                                    on:click=move |_| {
                                        override_chooser_open.set(true);
                                    }
                                >
                                    {t!(i18n, tournaments.creation.elimination.overrides.add)}
                                </button>
                            </div>

                            <Show when=move || override_chooser_open.get()>
                                <div class="flex flex-wrap gap-2 ui-setting-group">
                                    <For
                                        each=move || available_non_final_stages.get()
                                        key=|stage| *stage
                                        children=move |stage| {
                                            view! {
                                                <button
                                                    type="button"
                                                    class="ui-button ui-button-secondary ui-button-sm"
                                                    on:click=move |_| {
                                                        let available_stages = stages.get_untracked();
                                                        draft
                                                            .update(|draft| {
                                                                draft.add_override(&available_stages, stage)
                                                            });
                                                        override_chooser_open.set(false);
                                                    }
                                                >
                                                    {move || stage_label(i18n, stage)}
                                                </button>
                                            }
                                        }
                                    />
                                    <button
                                        type="button"
                                        class="ui-button ui-button-ghost ui-button-sm"
                                        on:click=move |_| override_chooser_open.set(false)
                                    >
                                        {t!(
                                            i18n, tournaments.creation.elimination.overrides.cancel
                                        )}
                                    </button>
                                </div>
                            </Show>

                            <For
                                each=move || {
                                    draft
                                        .with(|draft| {
                                            draft
                                                .overrides
                                                .iter()
                                                .enumerate()
                                                .filter_map(|(index, override_)| {
                                                    (!is_final_stage(override_.stage)).then_some(index)
                                                })
                                                .collect::<Vec<_>>()
                                        })
                                }
                                key=|index| *index
                                children=move |index| {
                                    view! {
                                        <StageOverrideEditor
                                            draft
                                            index
                                            tournament_clock
                                            expanded
                                        />
                                    }
                                }
                            />
                            <Show when=move || !invalid_stages.with(Vec::is_empty)>
                                <div class="ui-field-error">
                                    // TODO: i18n once copy is approved.
                                    <strong>"Check the match format for: "</strong>
                                    {move || {
                                        invalid_stages
                                            .get()
                                            .into_iter()
                                            .map(|stage| stage_label(i18n, stage))
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    }}
                                    // TODO: i18n once copy is approved.
                                    <span>
                                        " is unavailable with these players or this format. Adjust the players or remove this match format."
                                    </span>
                                </div>
                            </Show>
                        </details>
                    </div>
                </Show>

                <div class="space-y-3 ui-setting-group">
                    // TODO: i18n once copy is approved.
                    <h3 class="font-semibold text-gray-900 dark:text-gray-100">"Finals"</h3>
                    <Show
                        when=move || matches!(topology.get(), Topology::Single { .. })
                        fallback=move || {
                            view! {
                                // TODO: i18n once copy is approved.
                                <p class="ui-field-helper">
                                    "A second final is played if the lower-bracket winner wins the final."
                                </p>
                            }
                        }
                    >
                        <div class="flex gap-3 items-center">
                            <SimpleSwitch checked=bronze_match />
                            // TODO: i18n once copy is approved.
                            <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                "Play a third-place match"
                            </span>
                        </div>
                    </Show>
                    <Show when=move || draft.with(|draft| draft.advanced)>
                        <div class="space-y-3">
                            <button
                                type="button"
                                class="ui-button ui-button-secondary ui-button-sm"
                                prop:disabled=move || available_final_stages.with(Vec::is_empty)
                                on:click=move |_| final_override_chooser_open.set(true)
                            >
                                // TODO: i18n once copy is approved.
                                "Customize final"
                            </button>
                            <Show when=move || final_override_chooser_open.get()>
                                <div class="flex flex-wrap gap-2">
                                    <For
                                        each=move || available_final_stages.get()
                                        key=|stage| *stage
                                        children=move |stage| {
                                            view! {
                                                <button
                                                    type="button"
                                                    class="ui-button ui-button-secondary ui-button-sm"
                                                    on:click=move |_| {
                                                        let available_stages = stages.get_untracked();
                                                        draft
                                                            .update(|draft| {
                                                                draft.add_override(&available_stages, stage)
                                                            });
                                                        final_override_chooser_open.set(false);
                                                    }
                                                >
                                                    {move || stage_label(i18n, stage)}
                                                </button>
                                            }
                                        }
                                    />
                                    <button
                                        type="button"
                                        class="ui-button ui-button-ghost ui-button-sm"
                                        on:click=move |_| final_override_chooser_open.set(false)
                                    >
                                        // TODO: i18n once copy is approved.
                                        "Cancel"
                                    </button>
                                </div>
                            </Show>
                            <For
                                each=move || {
                                    draft
                                        .with(|draft| {
                                            draft
                                                .overrides
                                                .iter()
                                                .enumerate()
                                                .filter_map(|(index, override_)| {
                                                    is_final_stage(override_.stage).then_some(index)
                                                })
                                                .collect::<Vec<_>>()
                                        })
                                }
                                key=|index| *index
                                children=move |index| {
                                    view! {
                                        <StageOverrideEditor
                                            draft
                                            index
                                            tournament_clock
                                            expanded
                                        />
                                    }
                                }
                            />
                        </div>
                    </Show>
                </div>

            </Panel>
        </div>
    }
}

#[component]
fn StageOverrideEditor(
    draft: RwSignal<EliminationCreationDraft>,
    index: usize,
    tournament_clock: Signal<Clock>,
    expanded: RwSignal<Option<(PlanLocation, usize)>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let stage = move || draft.with(|draft| draft.overrides.get(index).map(|value| value.stage));

    view! {
        <details
            class="space-y-3 ui-setting-group"
            prop:open=move || {
                expanded
                    .get()
                    .is_some_and(|(location, _)| location == PlanLocation::Override(index))
            }
        >
            <summary class="font-semibold cursor-pointer">
                {move || stage().map(|stage| stage_label(i18n, stage))}
            </summary>
            <div class="flex flex-wrap gap-3 justify-between items-center">
                <div class="flex flex-col gap-1.5 grow min-w-48">
                    <span class="ui-field-label">
                        {t!(i18n, tournaments.creation.elimination.overrides.stage)}
                    </span>
                    <strong class="text-sm text-gray-900 dark:text-gray-100">
                        {move || stage().map(|stage| stage_label(i18n, stage))}
                    </strong>
                </div>
                <button
                    type="button"
                    class="ui-button ui-button-danger ui-button-sm"
                    on:click=move |_| {
                        draft
                            .update(|draft| {
                                if index < draft.overrides.len() {
                                    draft.overrides.remove(index);
                                    expanded
                                        .update(|active| {
                                            if let Some((PlanLocation::Override(current), _)) = active {
                                                if *current == index {
                                                    *active = None;
                                                } else if *current > index {
                                                    *current -= 1;
                                                }
                                            }
                                        });
                                }
                            });
                    }
                >
                    {t!(i18n, tournaments.creation.elimination.overrides.remove)}
                </button>
            </div>

            <SeriesPlanEditor
                draft
                location=PlanLocation::Override(index)
                tournament_clock
                expanded
            />
        </details>
    }
}

#[component]
fn SeriesPlanEditor(
    draft: RwSignal<EliminationCreationDraft>,
    location: PlanLocation,
    tournament_clock: Signal<Clock>,
    expanded: RwSignal<Option<(PlanLocation, usize)>>,
) -> impl IntoView {
    let plan = Signal::derive(move || {
        let clock = tournament_clock.get();
        draft
            .with(|draft| draft.plan(location).cloned())
            .ok_or(EliminationCreationError::EmptyPhaseList)?
            .to_plan(clock)
    });

    view! {
        <section class="space-y-4">
            <h3 class="font-semibold text-gray-900 dark:text-gray-100">
                // TODO: i18n once copy is approved.
                {move || match location {
                    PlanLocation::Default => String::from("Default match plan"),
                    PlanLocation::Override(_) => String::from("Stage match plan"),
                }}
            </h3>

            <div class="space-y-3">
                <div class="flex flex-wrap gap-3 justify-between items-center">
                    <div>
                        <span class="ui-field-label">
                            // TODO: i18n once copy is approved.
                            "Ordered steps"
                        </span>
                        <p class="ui-field-helper">
                            // TODO: i18n once copy is approved.
                            "A tied match moves to the next step. The final step repeats until decisive."
                        </p>
                    </div>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        prop:disabled=move || {
                            draft
                                .with(|draft| {
                                    draft
                                        .plan(location)
                                        .is_none_or(|plan| plan.phases.len() >= EDITOR_MAX_PHASES)
                                })
                        }
                        on:click=move |_| {
                            let clock = tournament_clock.get_untracked();
                            draft
                                .update(|draft| {
                                    if let Some(plan) = draft.plan_mut(location) {
                                        if plan.phases.len() < EDITOR_MAX_PHASES {
                                            plan.phases.push(EliminationPhaseDraft::balanced(clock));
                                            expanded.set(Some((location, plan.phases.len() - 1)));
                                        }
                                    }
                                });
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Add tiebreak step"
                    </button>
                </div>

                <For
                    each=move || {
                        0..draft
                            .with(|draft| {
                                draft.plan(location).map_or(0, |plan| plan.phases.len())
                            })
                    }
                    key=|index| *index
                    children=move |index| {
                        let phase = move || {
                            draft
                                .with(|draft| {
                                    draft
                                        .plan(location)
                                        .and_then(|plan| plan.phases.get(index))
                                        .cloned()
                                })
                        };
                        let is_last = move || {
                            draft
                                .with(|draft| {
                                    draft
                                        .plan(location)
                                        .is_some_and(|plan| index + 1 == plan.phases.len())
                                })
                        };
                        view! {
                            <div class="space-y-2 rounded border border-gray-200 dark:border-gray-700">
                                <button
                                    type="button"
                                    class="flex gap-2 justify-between items-center p-3 w-full text-left"
                                    aria-expanded=move || expanded.get() == Some((location, index))
                                    on:click=move |_| {
                                        expanded
                                            .update(|active| {
                                                *active = if *active == Some((location, index)) {
                                                    None
                                                } else {
                                                    Some((location, index))
                                                };
                                            })
                                    }
                                >
                                    <span class="min-w-0">
                                        // TODO: i18n once copy is approved.
                                        <strong class="text-sm">
                                            {if index == 0 {
                                                String::from("Step 1 · Initial games")
                                            } else {
                                                format!(
                                                    "Step {} · If {}tied",
                                                    index + 1,
                                                    if index > 1 { "still " } else { "" },
                                                )
                                            }}
                                        </strong>
                                        <span class="block text-xs text-gray-600 dark:text-gray-300">
                                            {move || {
                                                phase().map(|phase| phase_summary(&phase, is_last()))
                                            }}
                                        </span>
                                        <span class="block text-xs ui-field-error">
                                            {move || {
                                                phase()
                                                    .and_then(|phase| phase_error(
                                                        &phase,
                                                        is_last(),
                                                        tournament_clock.get(),
                                                    ))
                                            }}
                                        </span>
                                    </span>
                                    // TODO: i18n once copy is approved.
                                    <span class="text-xs">
                                        {move || {
                                            if expanded.get() == Some((location, index)) {
                                                "Close"
                                            } else {
                                                "Edit"
                                            }
                                        }}
                                    </span>
                                </button>
                                <Show when=move || expanded.get() == Some((location, index))>
                                    <PhaseEditor draft location index tournament_clock expanded />
                                </Show>
                            </div>
                        }
                    }
                />
            </div>

            <SeriesPreview plan />
        </section>
    }
}

// TODO: i18n once copy is approved.
fn phase_summary(phase: &EliminationPhaseDraft, is_last: bool) -> String {
    let play = match phase.clinch {
        ClinchPolicy::PlayAll => "play all games",
        ClinchPolicy::EarlyClinch => "stop when clinched",
    };
    let clock = if phase.use_tournament_clock {
        String::from("tournament clock")
    } else {
        match phase.clock {
            Clock::Realtime(clock) => format!(
                "{} + {}",
                format_compact_duration(clock.base_seconds.get()),
                format_compact_duration(clock.increment_seconds)
            ),
            Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each }) => {
                format!("{} each", format_compact_duration(seconds_each.get()))
            }
            Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move }) => {
                format!(
                    "{} per move",
                    format_compact_duration(seconds_per_move.get())
                )
            }
        }
    };
    let alternating = phase
        .color_order
        .windows(2)
        .all(|colors| colors[0] != colors[1]);
    let colors = if alternating {
        "alternating colors"
    } else {
        "custom color order"
    };
    let repeats = if is_last {
        String::from("repeat until decisive")
    } else {
        format!("up to {} game sets", phase.finite_sets)
    };
    format!(
        "{} games per set · {repeats} · {play} · {colors} · {clock}",
        phase.games_per_set
    )
}

fn phase_error(
    phase: &EliminationPhaseDraft,
    is_last: bool,
    tournament_clock: Clock,
) -> Option<String> {
    if !is_last && phase.finite_sets == 0 {
        return Some(EliminationCreationError::FiniteSetLimitMustBePositive.to_string());
    }
    EliminationPlanDraft {
        phases: vec![phase.clone()],
    }
    .to_plan(tournament_clock)
    .err()
    .map(|error| error.to_string())
}

#[component]
fn PhaseEditor(
    draft: RwSignal<EliminationCreationDraft>,
    location: PlanLocation,
    index: usize,
    tournament_clock: Signal<Clock>,
    expanded: RwSignal<Option<(PlanLocation, usize)>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let phase = move || {
        draft.with(|draft| {
            draft
                .plan(location)
                .and_then(|plan| plan.phases.get(index))
                .cloned()
        })
    };
    let is_last = move || {
        draft.with(|draft| {
            draft
                .plan(location)
                .is_some_and(|plan| index + 1 == plan.phases.len())
        })
    };
    let phase_count =
        move || draft.with(|draft| draft.plan(location).map_or(0, |plan| plan.phases.len()));

    view! {
        <div class="space-y-3 ui-setting-group">
            <div class="flex flex-wrap gap-2 justify-between items-center">
                <div class="flex flex-wrap gap-2">
                    <button
                        type="button"
                        class="ui-button ui-button-ghost ui-button-sm"
                        prop:disabled=index == 0
                        on:click=move |_| {
                            if index == 0 {
                                return;
                            }
                            draft
                                .update(|draft| {
                                    if let Some(plan) = draft.plan_mut(location) {
                                        if index < plan.phases.len() {
                                            plan.phases.swap(index - 1, index);
                                            expanded.set(Some((location, index - 1)));
                                        }
                                    }
                                });
                        }
                    >
                        {t!(i18n, tournaments.creation.elimination.phases.move_up)}
                    </button>
                    <button
                        type="button"
                        class="ui-button ui-button-ghost ui-button-sm"
                        prop:disabled=move || { index + 1 >= phase_count() }
                        on:click=move |_| {
                            draft
                                .update(|draft| {
                                    if let Some(plan) = draft.plan_mut(location) {
                                        if index + 1 < plan.phases.len() {
                                            plan.phases.swap(index, index + 1);
                                            expanded.set(Some((location, index + 1)));
                                        }
                                    }
                                });
                        }
                    >
                        {t!(i18n, tournaments.creation.elimination.phases.move_down)}
                    </button>
                    <button
                        type="button"
                        class="ui-button ui-button-danger ui-button-sm"
                        prop:disabled=move || { phase_count() <= 1 }
                        on:click=move |_| {
                            draft
                                .update(|draft| {
                                    if let Some(plan) = draft.plan_mut(location) {
                                        if plan.phases.len() > 1 && index < plan.phases.len() {
                                            plan.phases.remove(index);
                                            expanded.set(None);
                                        }
                                    }
                                });
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Remove step"
                    </button>
                </div>
            </div>

            <div class="grid gap-3 sm:grid-cols-3">
                <label class="flex flex-col gap-1.5">
                    <span class="ui-field-label">
                        // TODO: i18n once copy is approved.
                        "Games per game set"
                    </span>
                    <input
                        class="ui-field-input"
                        type="number"
                        min="1"
                        max=EDITOR_MAX_GAMES_PER_SET
                        prop:value=move || {
                            phase()
                                .map(|phase| phase.games_per_set)
                                .filter(|value| *value > 0)
                                .map(|value| value.to_string())
                                .unwrap_or_default()
                        }
                        on:input=move |event| {
                            let value = event_target_value(&event)
                                .parse::<u16>()
                                .ok()
                                .filter(|value| (1..=EDITOR_MAX_GAMES_PER_SET).contains(value))
                                .unwrap_or_default();
                            update_phase(
                                draft,
                                location,
                                index,
                                |phase| phase.set_games_per_set(value),
                            );
                        }
                    />
                </label>
                <Show
                    when=move || !is_last()
                    fallback=move || {
                        view! {
                            <div class="flex flex-col gap-1.5">
                                <span class="ui-field-label">
                                    // TODO: i18n once copy is approved.
                                    "Repeat limit"
                                </span>
                                <span class="ui-notice">
                                    // TODO: i18n once copy is approved.
                                    "Repeats until decisive"
                                </span>
                            </div>
                        }
                    }
                >
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            // TODO: i18n once copy is approved.
                            "Maximum game sets"
                        </span>
                        <input
                            class="ui-field-input"
                            type="number"
                            min="1"
                            max=EDITOR_MAX_FINITE_SETS
                            prop:value=move || {
                                phase()
                                    .map(|phase| phase.finite_sets)
                                    .filter(|value| *value > 0)
                                    .map(|value| value.to_string())
                                    .unwrap_or_default()
                            }
                            on:input=move |event| {
                                let value = event_target_value(&event)
                                    .parse::<u16>()
                                    .ok()
                                    .filter(|value| (1..=EDITOR_MAX_FINITE_SETS).contains(value))
                                    .unwrap_or_default();
                                update_phase(
                                    draft,
                                    location,
                                    index,
                                    |phase| phase.finite_sets = value,
                                );
                            }
                        />
                    </label>
                </Show>
                <label class="flex flex-col gap-1.5">
                    <span class="ui-field-label">
                        {t!(i18n, tournaments.creation.elimination.phases.clinch_policy)}
                    </span>
                    <select
                        class="ui-field-select"
                        prop:value=move || {
                            phase()
                                .map_or(
                                    "play_all",
                                    |phase| match phase.clinch {
                                        ClinchPolicy::PlayAll => "play_all",
                                        ClinchPolicy::EarlyClinch => "early_clinch",
                                    },
                                )
                        }
                        on:change=move |event| {
                            let clinch = match event_target_value(&event).as_str() {
                                "early_clinch" => ClinchPolicy::EarlyClinch,
                                _ => ClinchPolicy::PlayAll,
                            };
                            update_phase(draft, location, index, |phase| phase.clinch = clinch);
                        }
                    >
                        <option value="play_all">
                            {t!(i18n, tournaments.creation.elimination.phases.play_all)}
                        </option>
                        <option value="early_clinch">
                            {t!(i18n, tournaments.creation.elimination.phases.early_clinch)}
                        </option>
                    </select>
                </label>
            </div>

            <PhaseColorOrderEditor draft location index />
            <PhaseClockEditor draft location index tournament_clock />
        </div>
    }
}

#[component]
fn PhaseColorOrderEditor(
    draft: RwSignal<EliminationCreationDraft>,
    location: PlanLocation,
    index: usize,
) -> impl IntoView {
    let i18n = use_i18n();
    let phase = move || {
        draft.with(|draft| {
            draft
                .plan(location)
                .and_then(|plan| plan.phases.get(index))
                .cloned()
        })
    };
    let order_len = move || phase().map_or(0, |phase| phase.color_order.len());
    let inventory_valid = move || {
        phase().is_some_and(|phase| {
            if phase.color_order.len() != usize::from(phase.games_per_set) {
                return false;
            }
            let first = phase
                .color_order
                .iter()
                .filter(|side| **side == EntrantSide::First)
                .count();
            first.abs_diff(phase.color_order.len() - first) <= 1
        })
    };

    view! {
        <div class="space-y-2">
            <div>
                <span class="ui-field-label">
                    // TODO: i18n once copy is approved.
                    "Color order"
                </span>
                <p class="ui-field-helper">
                    // TODO: i18n once copy is approved.
                    "Set the order for one game set. Each repeated game set reverses this complete order."
                </p>
            </div>
            <div class="flex flex-wrap gap-2">
                <For
                    each=move || 0..order_len()
                    key=|color_index| *color_index
                    children=move |color_index| {
                        let color = move || {
                            phase().and_then(|phase| phase.color_order.get(color_index).copied())
                        };
                        let label = move || {
                            color().map(|color| color_label(i18n, color)).unwrap_or_default()
                        };
                        view! {
                            <div class="inline-flex gap-1 items-center p-1 rounded border border-gray-300 dark:border-gray-600">
                                <button
                                    type="button"
                                    class="ui-button ui-button-ghost ui-button-sm"
                                    prop:disabled=color_index == 0
                                    aria-label=move || {
                                        t_string!(
                                            i18n,
                                            tournaments.creation.elimination.colors.move_left,
                                            color = label(),
                                        )
                                            .to_string()
                                    }
                                    on:click=move |_| {
                                        if color_index == 0 {
                                            return;
                                        }
                                        update_phase(
                                            draft,
                                            location,
                                            index,
                                            |phase| {
                                                if color_index < phase.color_order.len() {
                                                    phase.color_order.swap(color_index - 1, color_index);
                                                }
                                            },
                                        );
                                    }
                                >
                                    "←"
                                </button>
                                <span class="px-1 text-sm text-center text-gray-900 dark:text-gray-100 min-w-12">
                                    {label}
                                </span>
                                <button
                                    type="button"
                                    class="ui-button ui-button-ghost ui-button-sm"
                                    prop:disabled=move || { color_index + 1 >= order_len() }
                                    aria-label=move || {
                                        t_string!(
                                            i18n,
                                            tournaments.creation.elimination.colors.move_right,
                                            color = label(),
                                        )
                                            .to_string()
                                    }
                                    on:click=move |_| {
                                        update_phase(
                                            draft,
                                            location,
                                            index,
                                            |phase| {
                                                if color_index + 1 < phase.color_order.len() {
                                                    phase.color_order.swap(color_index, color_index + 1);
                                                }
                                            },
                                        );
                                    }
                                >
                                    "→"
                                </button>
                            </div>
                        }
                    }
                />
            </div>
            <Show when=move || phase().is_some_and(|phase| phase.games_per_set % 2 == 0)>
                <p class="ui-field-helper">
                    {t!(i18n, tournaments.creation.elimination.colors.even_inventory)}
                </p>
            </Show>
            <Show when=move || phase().is_some_and(|phase| phase.games_per_set % 2 == 1)>
                <div class="flex flex-wrap gap-2 justify-between items-center ui-notice">
                    <span>
                        {move || match phase().and_then(|phase| phase.odd_color_advantage()) {
                            Some(EntrantSide::First) => {
                                t_string!(
                                    i18n,
                                tournaments.creation.elimination.colors.odd_advantage_first
                                )
                                    .to_string()
                            }
                            Some(EntrantSide::Second) => {
                                t_string!(
                                    i18n,
                                tournaments.creation.elimination.colors.odd_advantage_second
                                )
                                    .to_string()
                            }
                            None => String::new(),
                        }}
                    </span>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        on:click=move |_| {
                            let next = match phase().and_then(|phase| phase.odd_color_advantage()) {
                                Some(EntrantSide::First) => EntrantSide::Second,
                                Some(EntrantSide::Second) | None => EntrantSide::First,
                            };
                            update_phase(
                                draft,
                                location,
                                index,
                                |phase| { phase.give_odd_color_advantage_to(next) },
                            );
                        }
                    >
                        {move || match phase().and_then(|phase| phase.odd_color_advantage()) {
                            Some(EntrantSide::First) => {
                                t_string!(
                                    i18n,
                                tournaments.creation.elimination.colors.give_extra_second
                                )
                                    .to_string()
                            }
                            Some(EntrantSide::Second) | None => {
                                t_string!(
                                    i18n,
                                tournaments.creation.elimination.colors.give_extra_first
                                )
                                    .to_string()
                            }
                        }}
                    </button>
                </div>
            </Show>
            <Show when=move || !inventory_valid()>
                <small class="ui-field-error">
                    {t!(i18n, tournaments.creation.elimination.colors.invalid_inventory)}
                </small>
            </Show>
        </div>
    }
}

#[component]
fn PhaseClockEditor(
    draft: RwSignal<EliminationCreationDraft>,
    location: PlanLocation,
    index: usize,
    tournament_clock: Signal<Clock>,
) -> impl IntoView {
    let i18n = use_i18n();
    let clock = move || {
        draft.with(|draft| {
            draft
                .plan(location)
                .and_then(|plan| plan.phases.get(index))
                .map(|phase| phase.clock)
        })
    };
    let is_realtime = move || clock().is_some_and(|clock| matches!(clock, Clock::Realtime(_)));
    let is_correspondence =
        move || clock().is_some_and(|clock| matches!(clock, Clock::Correspondence(_)));
    let uses_tournament_clock = move || {
        draft.with(|draft| {
            draft
                .plan(location)
                .and_then(|plan| plan.phases.get(index))
                .is_some_and(|phase| phase.use_tournament_clock)
        })
    };

    view! {
        <div class="space-y-2">
            <label class="flex flex-col gap-1.5 max-w-sm">
                // TODO: i18n once copy is approved.
                <span class="ui-field-label">"Clock"</span>
                <select
                    class="ui-field-select"
                    prop:value=move || if uses_tournament_clock() { "tournament" } else { "custom" }
                    on:change=move |event| {
                        let use_tournament_clock = event_target_value(&event) == "tournament";
                        let inherited_clock = tournament_clock.get_untracked();
                        update_phase(
                            draft,
                            location,
                            index,
                            |phase| {
                                phase.use_tournament_clock = use_tournament_clock;
                                phase.clock = inherited_clock;
                            },
                        );
                    }
                >
                    // TODO: i18n once copy is approved.
                    <option value="tournament">"Use tournament clock"</option>
                    // TODO: i18n once copy is approved.
                    <option value="custom">"Custom clock"</option>
                </select>
            </label>
            <Show when=move || !uses_tournament_clock() && is_realtime()>
                <div class="grid gap-3 sm:grid-cols-2">
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            {t!(i18n, tournaments.creation.elimination.clock.base_minutes)}
                        </span>
                        <input
                            class="ui-field-input"
                            type="number"
                            min="1"
                            max="1440"
                            prop:value=move || match clock() {
                                Some(Clock::Realtime(clock)) => clock.base_seconds.get() / 60,
                                _ => 1,
                            }
                            on:input=move |event| {
                                let Ok(minutes) = event_target_value(&event).parse::<u32>() else {
                                    return;
                                };
                                if !(1..=1440).contains(&minutes) {
                                    return;
                                }
                                let Some(seconds) = minutes
                                    .checked_mul(60)
                                    .and_then(NonZeroU32::new) else {
                                    return;
                                };
                                update_phase(
                                    draft,
                                    location,
                                    index,
                                    |phase| {
                                        if let Clock::Realtime(clock) = &mut phase.clock {
                                            clock.base_seconds = seconds;
                                        }
                                    },
                                );
                            }
                        />
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            {t!(i18n, tournaments.creation.elimination.clock.increment_seconds)}
                        </span>
                        <input
                            class="ui-field-input"
                            type="number"
                            min="0"
                            max="3600"
                            prop:value=move || match clock() {
                                Some(Clock::Realtime(clock)) => clock.increment_seconds,
                                _ => 0,
                            }
                            on:input=move |event| {
                                let Ok(seconds) = event_target_value(&event).parse::<u32>() else {
                                    return;
                                };
                                if seconds > 3600 {
                                    return;
                                }
                                update_phase(
                                    draft,
                                    location,
                                    index,
                                    |phase| {
                                        if let Clock::Realtime(clock) = &mut phase.clock {
                                            clock.increment_seconds = seconds;
                                        }
                                    },
                                );
                            }
                        />
                    </label>
                </div>
            </Show>
            <Show when=move || !uses_tournament_clock() && is_correspondence()>
                <div class="grid gap-3 sm:grid-cols-2">
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            {t!(
                                i18n, tournaments.creation.elimination.clock.correspondence_control
                            )}
                        </span>
                        <select
                            class="ui-field-select"
                            prop:value=move || match clock() {
                                Some(
                                    Clock::Correspondence(CorrespondenceClock::DaysPerMove { .. }),
                                ) => "days_per_move",
                                _ => "total_time_each",
                            }
                            on:change=move |event| {
                                let control = event_target_value(&event);
                                let days = clock().map_or(2, correspondence_days).max(1);
                                let seconds = NonZeroU32::new(days.saturating_mul(SECONDS_PER_DAY))
                                    .expect("positive days remain positive");
                                update_phase(
                                    draft,
                                    location,
                                    index,
                                    |phase| {
                                        phase.clock = Clock::Correspondence(
                                            if control == "days_per_move" {
                                                CorrespondenceClock::DaysPerMove {
                                                    seconds_per_move: seconds,
                                                }
                                            } else {
                                                CorrespondenceClock::TotalTimeEach {
                                                    seconds_each: seconds,
                                                }
                                            },
                                        );
                                    },
                                );
                            }
                        >
                            <option value="days_per_move">
                                {t!(i18n, tournaments.creation.elimination.clock.days_per_move)}
                            </option>
                            <option value="total_time_each">
                                {t!(i18n, tournaments.creation.elimination.clock.total_days_each)}
                            </option>
                        </select>
                    </label>
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">
                            {t!(i18n, tournaments.creation.elimination.clock.days)}
                        </span>
                        <input
                            class="ui-field-input"
                            type="number"
                            min="1"
                            max="365"
                            prop:value=move || clock().map_or(1, correspondence_days)
                            on:input=move |event| {
                                let Ok(days) = event_target_value(&event).parse::<u32>() else {
                                    return;
                                };
                                if !(1..=365).contains(&days) {
                                    return;
                                }
                                let seconds = NonZeroU32::new(days * SECONDS_PER_DAY)
                                    .expect("positive bounded days remain positive");
                                update_phase(
                                    draft,
                                    location,
                                    index,
                                    |phase| {
                                        if let Clock::Correspondence(clock) = &mut phase.clock {
                                            *clock = match clock {
                                                CorrespondenceClock::DaysPerMove { .. } => {
                                                    CorrespondenceClock::DaysPerMove {
                                                        seconds_per_move: seconds,
                                                    }
                                                }
                                                CorrespondenceClock::TotalTimeEach { .. } => {
                                                    CorrespondenceClock::TotalTimeEach {
                                                        seconds_each: seconds,
                                                    }
                                                }
                                            };
                                        }
                                    },
                                );
                            }
                        />
                    </label>
                </div>
            </Show>
            <Show when=move || {
                let tournament_clock = tournament_clock.get();
                clock().is_some_and(|clock| clock.mode() != tournament_clock.mode())
            }>
                <small class="ui-field-error">
                    {t!(i18n, tournaments.creation.elimination.clock.mode_mismatch)}
                </small>
            </Show>
        </div>
    }
}

#[component]
fn SeriesPreview(plan: Signal<Result<SeriesPlan, EliminationCreationError>>) -> impl IntoView {
    let i18n = use_i18n();
    let open = RwSignal::new(false);
    view! {
        <details class="ui-setting-group" on:toggle=move |_| open.update(|open| *open = !*open)>
            <summary class="font-semibold text-gray-900 cursor-pointer dark:text-gray-100">
                {t!(i18n, tournaments.creation.elimination.preview.title)}
            </summary>
            <Show when=move || open.get()>
                <div class="overflow-auto mt-3 space-y-1 max-h-96 text-sm">
                    {move || match plan.get().and_then(|plan| preview_plan(&plan)) {
                        Ok(items) => {
                            items
                                .into_iter()
                                .map(|item| view! { <PreviewItemView item /> })
                                .collect_view()
                                .into_any()
                        }
                        Err(error) => {
                            view! {
                                <small class="ui-field-error">
                                    {move || elimination_error_text(i18n, error.clone())}
                                </small>
                            }
                                .into_any()
                        }
                    }}
                </div>
            </Show>
        </details>
    }
}

#[component]
fn PreviewItemView(item: PreviewItem) -> impl IntoView {
    let i18n = use_i18n();
    match item {
        PreviewItem::Phase {
            phase_ordinal,
            games_per_set,
            set_limit,
            clinch,
            clock,
        } => {
            let text = move || {
                let clinch = match clinch {
                    ClinchPolicy::PlayAll => t_string!(
                        i18n,
                        tournaments.creation.elimination.preview.clinch_play_all
                    )
                    .to_string(),
                    ClinchPolicy::EarlyClinch => {
                        t_string!(i18n, tournaments.creation.elimination.preview.clinch_early)
                            .to_string()
                    }
                };
                let clock = localized_clock_text(i18n, clock);
                match set_limit {
                    SetLimit::AtMost(sets) => t_string!(
                        i18n,
                        tournaments.creation.elimination.preview.phase_finite,
                        phase = phase_ordinal,
                        games = games_per_set,
                        sets = sets.get(),
                        clinch = clinch,
                        clock = clock,
                    )
                    .to_string(),
                    SetLimit::UntilDecisive => t_string!(
                        i18n,
                        tournaments.creation.elimination.preview.phase_until,
                        phase = phase_ordinal,
                        games = games_per_set,
                        clinch = clinch,
                        clock = clock,
                    )
                    .to_string(),
                }
            };
            view! { <p class="pt-2 font-semibold text-gray-900 dark:text-gray-100">{text}</p> }
                .into_any()
        }
        PreviewItem::Game {
            phase_ordinal,
            set_ordinal,
            game_ordinal_in_set,
            game_ordinal_in_series,
            white,
        } => {
            let text = move || {
                let first = t_string!(i18n, tournaments.creation.elimination.preview.first_entrant)
                    .to_string();
                let second = t_string!(
                    i18n,
                    tournaments.creation.elimination.preview.second_entrant
                )
                .to_string();
                let (white, black) = match white {
                    EntrantSide::First => (first, second),
                    EntrantSide::Second => (second, first),
                };
                t_string!(
                    i18n,
                    tournaments.creation.elimination.preview.game,
                    series_game = game_ordinal_in_series,
                    phase = phase_ordinal,
                    set = set_ordinal,
                    set_game = game_ordinal_in_set,
                    white = white,
                    black = black,
                )
                .to_string()
            };
            view! { <p class="text-gray-700 dark:text-gray-300">{text}</p> }.into_any()
        }
        PreviewItem::RepeatedFiniteSets {
            phase_ordinal,
            additional_sets,
        } => {
            // TODO: i18n once copy is approved.
            let text = format!(
                "If still tied, phase {phase_ordinal} repeats for {additional_sets} more set(s), flipping every White assignment each set."
            );
            view! { <p class="font-medium text-amber-700 dark:text-amber-300">{text}</p> }
                .into_any()
        }
        PreviewItem::TieTransition {
            from_phase,
            to_phase,
        } => view! {
            <p class="font-medium text-amber-700 dark:text-amber-300">
                {move || {
                    t_string!(
                        i18n,
                        tournaments.creation.elimination.preview.next_phase,
                        from_phase = from_phase,
                        to_phase = to_phase,
                    )
                        .to_string()
                }}
            </p>
        }
        .into_any(),
        PreviewItem::RepeatUntilDecisive { phase_ordinal } => {
            // TODO: i18n once copy is approved.
            let text = format!(
                "If tied, phase {phase_ordinal} repeats until decisive, flipping every White assignment each set."
            );
            view! { <p class="font-medium text-amber-700 dark:text-amber-300">{text}</p> }
                .into_any()
        }
    }
}

fn update_phase(
    draft: RwSignal<EliminationCreationDraft>,
    location: PlanLocation,
    index: usize,
    update: impl FnOnce(&mut EliminationPhaseDraft),
) {
    draft.update(|draft| {
        let Some(phase) = draft
            .plan_mut(location)
            .and_then(|plan| plan.phases.get_mut(index))
        else {
            return;
        };
        update(phase);
    });
}

fn elimination_error_text(
    i18n: I18nContext<Locale, I18nKeys>,
    error: EliminationCreationError,
) -> String {
    match error {
        EliminationCreationError::EmptyPhaseList => {
            t_string!(i18n, tournaments.creation.elimination.errors.empty_phases)
        }
        EliminationCreationError::GamesPerSetMustBePositive => {
            t_string!(i18n, tournaments.creation.elimination.errors.games_positive)
        }
        EliminationCreationError::ColorOrderLengthMismatch { .. } => {
            t_string!(
                i18n,
                tournaments.creation.elimination.colors.invalid_inventory
            )
        }
        EliminationCreationError::InvalidColorInventory { .. } => {
            t_string!(
                i18n,
                tournaments.creation.elimination.colors.invalid_inventory
            )
        }
        EliminationCreationError::FiniteSetLimitMustBePositive => t_string!(
            i18n,
            tournaments.creation.elimination.errors.finite_positive
        ),
        EliminationCreationError::MixedTimeModes => {
            t_string!(i18n, tournaments.creation.elimination.errors.same_time_mode)
        }
        EliminationCreationError::InvalidStageOverrides(_) => {
            // TODO: i18n once copy is approved.
            "Remove or repair the invalid stage overrides."
        }
        EliminationCreationError::PreviewOrdinalOverflow => t_string!(
            i18n,
            tournaments.creation.elimination.errors.preview_overflow
        ),
    }
    .to_string()
}

fn localized_clock_text(i18n: I18nContext<Locale, I18nKeys>, clock: Clock) -> String {
    match clock {
        Clock::Realtime(clock) => t_string!(
            i18n,
            tournaments.creation.elimination.clock.realtime,
            minutes = clock.base_seconds.get() / 60,
            increment = clock.increment_seconds,
        ),
        Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move }) => t_string!(
            i18n,
            tournaments.creation.elimination.clock.days_per_move_summary,
            days = seconds_per_move.get() / SECONDS_PER_DAY,
        ),
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each }) => t_string!(
            i18n,
            tournaments
                .creation
                .elimination
                .clock
                .total_days_each_summary,
            days = seconds_each.get() / SECONDS_PER_DAY,
        ),
    }
    .to_string()
}

fn correspondence_days(clock: Clock) -> u32 {
    match clock {
        Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move }) => {
            seconds_per_move.get() / SECONDS_PER_DAY
        }
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each }) => {
            seconds_each.get() / SECONDS_PER_DAY
        }
        Clock::Realtime(_) => 1,
    }
}

fn color_label(i18n: I18nContext<Locale, I18nKeys>, color: EntrantSide) -> String {
    match color {
        EntrantSide::First => {
            t_string!(i18n, tournaments.creation.elimination.colors.first_short).to_string()
        }
        EntrantSide::Second => {
            t_string!(i18n, tournaments.creation.elimination.colors.second_short).to_string()
        }
    }
}

fn stage_label(i18n: I18nContext<Locale, I18nKeys>, stage: Stage) -> String {
    let (label, round_index) = match stage {
        Stage::SingleRound { round_index } => (
            t_string!(i18n, tournaments.creation.elimination.stages.single_round).to_string(),
            Some(round_index),
        ),
        Stage::SingleFinal => (
            t_string!(i18n, tournaments.creation.elimination.stages.single_final).to_string(),
            None,
        ),
        Stage::Bronze => (
            t_string!(i18n, tournaments.creation.elimination.stages.bronze).to_string(),
            None,
        ),
        Stage::WinnersRound { round_index } => (
            t_string!(i18n, tournaments.creation.elimination.stages.winners_round).to_string(),
            Some(round_index),
        ),
        Stage::LosersMinor { round_index } => (
            t_string!(i18n, tournaments.creation.elimination.stages.losers_minor).to_string(),
            Some(round_index),
        ),
        Stage::LosersMajor { round_index } => (
            t_string!(i18n, tournaments.creation.elimination.stages.losers_major).to_string(),
            Some(round_index),
        ),
        Stage::GrandFinal => (
            // TODO: i18n once copy is approved.
            String::from("Final"),
            None,
        ),
        Stage::Reset => (
            // TODO: i18n once copy is approved.
            String::from("Second final"),
            None,
        ),
    };
    round_index.map_or(label.clone(), |round_index| {
        format!("{label} {}", round_index + 1)
    })
}

const fn is_final_stage(stage: Stage) -> bool {
    matches!(
        stage,
        Stage::SingleFinal | Stage::Bronze | Stage::GrandFinal | Stage::Reset
    )
}
