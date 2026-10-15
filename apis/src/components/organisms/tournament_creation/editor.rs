use super::model::{
    EliminationCreationDraft,
    EliminationCreationError,
    EliminationPhaseDraft,
    EliminationPlanDraft,
    PlanLocation,
    EDITOR_MAX_FINITE_SETS,
    EDITOR_MAX_GAMES_PER_SET,
    EDITOR_MAX_PHASES,
};
use crate::{
    common::compact_realtime_clock,
    components::{
        atoms::simple_switch::SimpleSwitch,
        molecules::{panel::Panel, time_row::format_compact_duration},
    },
    i18n::*,
};
use leptos::{html, prelude::*};
use leptos_i18n::I18nContext;
use shared_types::tournament::{
    elimination::{ClinchPolicy, EntrantSide, Topology},
    Clock,
    CorrespondenceClock,
};
use std::num::NonZeroU32;

const SECONDS_PER_DAY: u32 = 86_400;

#[component]
pub(crate) fn EliminationCreationEditor(
    draft: RwSignal<EliminationCreationDraft>,
    topology: Signal<Topology>,
    tournament_clock: Signal<Clock>,
    bronze_match: RwSignal<bool>,
    reveal_invalid: RwSignal<u32>,
    children: Children,
) -> impl IntoView {
    let custom_expanded = RwSignal::new(Some(0_usize));
    let finals_expanded = RwSignal::new(Some(0_usize));
    let focus_custom = RwSignal::new(0_u32);
    let focus_finals = RwSignal::new(0_u32);
    Effect::new(move |_| {
        if reveal_invalid.get() == 0 {
            return;
        }
        let clock = tournament_clock.get_untracked();
        let invalid = draft.with_untracked(|draft| {
            [PlanLocation::Custom, PlanLocation::Finals]
                .into_iter()
                .filter(|location| *location != PlanLocation::Custom || draft.custom)
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
        if let Some((location, index)) = invalid {
            let (expanded, focus) = match location {
                PlanLocation::Custom => (custom_expanded, focus_custom),
                PlanLocation::Finals => (finals_expanded, focus_finals),
            };
            expanded.set(Some(index));
            focus.update(|value| *value += 1);
        }
    });
    let custom = Signal::derive(move || draft.with(|draft| draft.custom));
    let has_finals = move || draft.with(|draft| draft.finals.is_some());
    // TODO: i18n once copy is approved.
    let finals_scope = move || match topology.get() {
        Topology::Single { bronze: false } => "Applies to the final.",
        Topology::Single { bronze: true } => "Applies to the final and third-place match.",
        Topology::Double => "Applies to the grand final and reset final, if needed.",
    };

    view! {
        <Panel
            // TODO: i18n once copy is approved.
            title="Match format"
            body_class="space-y-5"
        >
            {children()}
            <div class="grid gap-3 sm:grid-cols-2">
                // TODO: i18n once copy is approved.
                <PlanChoice
                    label="Balanced"
                    description="Two-game sets"
                    active=Signal::derive(move || !custom.get())
                    on_select=Callback::new(move |()| draft.update(|draft| draft.custom = false))
                />
                // TODO: i18n once copy is approved.
                <PlanChoice
                    label="Custom"
                    description="Choose match steps"
                    active=custom
                    on_select=Callback::new(move |()| draft.update(|draft| draft.custom = true))
                />
            </div>
            <Show
                when=move || custom.get()
                fallback=move || {
                    view! {
                        // TODO: i18n once copy is approved.
                        <p class="ui-field-helper">
                            "Play two games, one as each color. If tied, repeat with colors reversed."
                        </p>
                    }
                }
            >
                <SeriesPlanEditor
                    draft
                    location=PlanLocation::Custom
                    tournament_clock
                    expanded=custom_expanded
                    focus_editor=focus_custom
                />
            </Show>

            <section class="pt-5 space-y-4 border-t border-gray-200 dark:border-gray-700">
                <div class="flex flex-wrap gap-3 justify-between items-center">
                    <div class="flex flex-wrap gap-3 items-center">
                        // TODO: i18n once copy is approved.
                        <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">
                            "Finals"
                        </h3>
                        <Show when=has_finals>
                            // TODO: i18n once copy is approved.
                            <span class="py-1 px-2 text-xs font-medium rounded border border-pillbug-teal text-pillbug-teal">
                                "Custom format"
                            </span>
                        </Show>
                    </div>
                    <Show when=has_finals>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-sm"
                            on:click=move |_| draft.update(|draft| draft.finals = None)
                        >
                            // TODO: i18n once copy is approved.
                            "Use match format"
                        </button>
                    </Show>
                </div>
                <Show
                    when=move || matches!(topology.get(), Topology::Single { .. })
                    fallback=move || {
                        view! {
                            // TODO: i18n once copy is approved.
                            <p class="ui-field-helper">
                                "A reset final is played if the lower-bracket winner wins the grand final."
                            </p>
                        }
                    }
                >
                    // TODO: i18n once copy is approved.
                    <div
                        class="flex gap-3 items-center"
                        role="group"
                        aria-label="Play a third-place match"
                    >
                        <SimpleSwitch checked=bronze_match />
                        // TODO: i18n once copy is approved.
                        <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                            "Play a third-place match"
                        </span>
                    </div>
                </Show>
                <Show
                    when=has_finals
                    fallback=move || {
                        view! {
                            <div class="flex flex-col gap-3 sm:flex-row sm:justify-between sm:items-center ui-setting-group">
                                <div class="space-y-1">
                                    // TODO: i18n once copy is approved.
                                    <strong class="text-sm">"Uses match format"</strong>
                                    // TODO: i18n once copy is approved.
                                    <p class="ui-field-helper">
                                        "Uses the format selected above. "{finals_scope}
                                    </p>
                                </div>
                                <button
                                    type="button"
                                    class="shrink-0 ui-button ui-button-secondary ui-button-sm"
                                    on:click=move |_| {
                                        draft
                                            .update(|draft| {
                                                draft.customize_final(tournament_clock.get_untracked())
                                            });
                                        finals_expanded.set(Some(0));
                                    }
                                >
                                    // TODO: i18n once copy is approved.
                                    "Customize final"
                                </button>
                            </div>
                        }
                    }
                >
                    <p class="ui-field-helper">{finals_scope}</p>
                    <SeriesPlanEditor
                        draft
                        location=PlanLocation::Finals
                        tournament_clock
                        expanded=finals_expanded
                        focus_editor=focus_finals
                    />
                </Show>
            </section>
        </Panel>
    }
}

#[component]
fn PlanChoice(
    label: &'static str,
    description: &'static str,
    active: Signal<bool>,
    #[prop(optional)] disabled: Signal<bool>,
    on_select: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class="flex gap-3 items-center p-3 min-w-0 text-left rounded-lg border transition-colors disabled:opacity-50 disabled:cursor-not-allowed focus-visible:outline-pillbug-teal"
            class:border-pillbug-teal=move || active.get()
            class=("bg-pillbug-teal/10", move || active.get())
            class:border-gray-300=move || !active.get()
            class=("dark:border-gray-600", move || !active.get())
            aria-pressed=move || active.get().to_string()
            prop:disabled=disabled
            on:click=move |_| on_select.run(())
        >
            <span
                aria-hidden="true"
                class="flex justify-center items-center rounded-full border-2 shrink-0 size-5"
                class:border-pillbug-teal=move || active.get()
                class:text-pillbug-teal=move || active.get()
                class:border-gray-400=move || !active.get()
            >
                <Show when=move || active.get()>
                    <span class="rounded-full size-2.5 bg-pillbug-teal"></span>
                </Show>
            </span>
            <span class="space-y-1">
                <strong class="block text-sm text-gray-900 dark:text-gray-100">{label}</strong>
                <span class="block text-sm text-gray-600 dark:text-gray-300">{description}</span>
            </span>
        </button>
    }
}

#[component]
fn SeriesPlanEditor(
    draft: RwSignal<EliminationCreationDraft>,
    location: PlanLocation,
    tournament_clock: Signal<Clock>,
    expanded: RwSignal<Option<usize>>,
    focus_editor: RwSignal<u32>,
) -> impl IntoView {
    view! {
        <div class="space-y-3">
            <For
                each=move || {
                    0..draft.with(|draft| draft.plan(location).map_or(0, |plan| plan.phases.len()))
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
                    let set_limit_input = NodeRef::<html::Input>::new();
                    Effect::new(move |_| {
                        if focus_editor.get() == 0 || expanded.get_untracked() != Some(index) {
                            return;
                        }
                        if untrack(|| {
                            !is_last() && phase().is_some_and(|phase| phase.finite_sets == 0)
                        }) {
                            if let Some(input) = set_limit_input.get() {
                                let _ = input.focus();
                            }
                        }
                    });
                    view! {
                        <div class="rounded-lg border border-gray-200 dark:border-gray-700">
                            <button
                                type="button"
                                class="flex gap-3 justify-between items-center p-3 w-full text-left"
                                aria-expanded=move || (expanded.get() == Some(index)).to_string()
                                on:click=move |_| {
                                    expanded
                                        .update(|active| {
                                            *active = if *active == Some(index) {
                                                None
                                            } else {
                                                Some(index)
                                            };
                                        })
                                }
                            >
                                <span class="space-y-1 min-w-0">
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
                                        if expanded.get() == Some(index) { "Close" } else { "Edit" }
                                    }}
                                </span>
                            </button>
                            <Show when=move || expanded.get() == Some(index)>
                                <PhaseEditor
                                    draft
                                    location
                                    index
                                    tournament_clock
                                    expanded
                                    focus_editor
                                />
                            </Show>
                        </div>
                        <Show when=move || !is_last()>
                            <div class="pl-3 ml-3 space-y-1 border-l border-gray-400 border-dashed">
                                <label class="block text-sm text-gray-700 dark:text-gray-300">
                                    // TODO: i18n once copy is approved.
                                    "If still tied after "
                                    <input
                                        node_ref=set_limit_input
                                        class="inline-block mx-1 align-middle max-w-20 ui-field-input"
                                        type="number"
                                        min="1"
                                        max=EDITOR_MAX_FINITE_SETS
                                        // TODO: i18n once copy is approved.
                                        aria-label=move || {
                                            format!("Total set limit for step {}", index + 1)
                                        }
                                        aria-invalid=move || {
                                            phase()
                                                .is_some_and(|phase| phase.finite_sets == 0)
                                                .to_string()
                                        }
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
                                                .filter(|value| {
                                                    (1..=EDITOR_MAX_FINITE_SETS).contains(value)
                                                })
                                                .unwrap_or_default();
                                            update_phase(
                                                draft,
                                                location,
                                                index,
                                                |phase| phase.finite_sets = value,
                                            );
                                        }
                                    /> // TODO: i18n once copy is approved.
                                    {move || {
                                        if phase().is_some_and(|phase| phase.finite_sets == 1) {
                                            " set, use the next settings."
                                        } else {
                                            " sets, use the next settings."
                                        }
                                    }}
                                </label>
                                // TODO: i18n once copy is approved.
                                <p class="ui-field-helper">"Includes the initial set."</p>
                            </div>
                        </Show>
                    }
                }
            />
        </div>
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
            Clock::Realtime(clock) => compact_realtime_clock(clock),
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
        format!(
            "up to {} {} total",
            phase.finite_sets,
            if phase.finite_sets == 1 {
                "set"
            } else {
                "sets"
            }
        )
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
    expanded: RwSignal<Option<usize>>,
    focus_editor: RwSignal<u32>,
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

    let games_input = NodeRef::<html::Input>::new();
    Effect::new(move |_| {
        focus_editor.track();
        if untrack(|| !is_last() && phase().is_some_and(|phase| phase.finite_sets == 0)) {
            return;
        }
        if let Some(input) = games_input.get() {
            let _ = input.focus();
        }
    });

    view! {
        <div class="p-3 pt-0 space-y-4">
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
                                            expanded.set(Some(index - 1));
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
                                            expanded.set(Some(index + 1));
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

            <div class="grid gap-3 sm:grid-cols-2">
                <label class="flex flex-col gap-1.5">
                    <span class="ui-field-label">
                        // TODO: i18n once copy is approved.
                        "Games per set"
                    </span>
                    <input
                        node_ref=games_input
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
                <label class="flex flex-col gap-1.5">
                    <span class="ui-field-label">
                        // TODO: i18n once copy is approved.
                        "Finish rule"
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
            <Show when=is_last>
                <div class="pt-3 space-y-2 border-t border-gray-200 dark:border-gray-700">
                    // TODO: i18n once copy is approved.
                    <h4 class="text-sm font-semibold">"If tied"</h4>
                    <div class="grid gap-3 sm:grid-cols-2">
                        // TODO: i18n once copy is approved.
                        <PlanChoice
                            label="Repeat these games"
                            description="Keep these settings until there is a winner."
                            active=Signal::derive(|| true)
                            on_select=Callback::new(|()| {})
                        />
                        // TODO: i18n once copy is approved.
                        <PlanChoice
                            label="Use different settings"
                            description="Continue with another set of games."
                            active=Signal::derive(|| false)
                            disabled=Signal::derive(move || phase_count() >= EDITOR_MAX_PHASES)
                            on_select=Callback::new(move |()| {
                                draft
                                    .update(|draft| {
                                        if let Some(next) = draft
                                            .plan_mut(location)
                                            .and_then(|plan| plan.extend_if_tied())
                                        {
                                            expanded.set(Some(next));
                                        }
                                    });
                            })
                        />
                    </div>
                    <Show when=move || { phase_count() >= EDITOR_MAX_PHASES }>
                        // TODO: i18n once copy is approved.
                        <p class="ui-field-helper">"A match can have up to eight steps."</p>
                    </Show>
                </div>
            </Show>
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
                    "Who plays White"
                </span>
                <p class="ui-field-helper">
                    // TODO: i18n once copy is approved.
                    "First and Second are the two entrants. Repeated sets swap colors."
                </p>
            </div>
            <div class="flex overflow-x-auto gap-2 pb-1 min-h-20">
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
                            <div class="flex flex-col items-center p-1 rounded border border-gray-300 dark:border-gray-600 shrink-0">
                                // TODO: i18n once copy is approved.
                                <span class="text-xs text-gray-500">
                                    {format!("Game {}", color_index + 1)}
                                </span>
                                <div class="inline-flex items-center">
                                    <button
                                        type="button"
                                        class="ui-button ui-button-ghost ui-button-icon-sm"
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
                                        class="ui-button ui-button-ghost ui-button-icon-sm"
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
                            </div>
                        }
                    }
                />
            </div>
            <Show when=move || {
                phase().is_some_and(|phase| phase.games_per_set > 0 && phase.games_per_set % 2 == 0)
            }>
                <p
                    class="flex items-center h-10 ui-field-helper"
                    title=move || {
                        t_string!(i18n, tournaments.creation.elimination.colors.even_inventory)
                            .to_string()
                    }
                >
                    // TODO: i18n once copy is approved.
                    "Equal White games"
                </p>
            </Show>
            <Show when=move || phase().is_some_and(|phase| phase.games_per_set % 2 == 1)>
                <div class="flex gap-2 justify-between items-center h-10">
                    <span
                        class="text-sm"
                        title=move || match phase().and_then(|phase| phase.odd_color_advantage()) {
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
                        }
                    >
                        // TODO: i18n once copy is approved.
                        {move || match phase().and_then(|phase| phase.odd_color_advantage()) {
                            Some(EntrantSide::First) => "Extra White: First",
                            Some(EntrantSide::Second) => "Extra White: Second",
                            None => "",
                        }}
                    </span>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        title=move || match phase().and_then(|phase| phase.odd_color_advantage()) {
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
                        }
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
                        // TODO: i18n once copy is approved.
                        "Swap"

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
