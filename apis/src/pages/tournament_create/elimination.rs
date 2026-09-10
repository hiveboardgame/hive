use super::common::{
    clock_signal,
    validate_configuration,
    ClockFields,
    CommonDraft,
    CreationDetails,
    CreationShell,
    FieldDraft,
    FieldSizeFields,
};
use crate::{
    common::TimeParamsStoreFields,
    components::organisms::tournament_creation::{
        build_elimination_creation_configuration,
        elimination_stages_for_field,
        EliminationCreationDraft,
        EliminationCreationEditor,
    },
    i18n::*,
    providers::{ChallengeParams, ChallengeParamsStoreFields},
};
use leptos::prelude::*;
use reactive_stores::Store;
use shared_types::{
    tournament::{
        elimination::{Stage, Topology},
        Clock,
        Config,
        Format,
        MAX_ELIMINATION_SEATS,
    },
    TimeMode,
};

#[derive(Clone, Copy)]
struct EliminationDraft {
    field: FieldDraft,
    double: RwSignal<bool>,
    bronze: RwSignal<bool>,
    plan: RwSignal<EliminationCreationDraft>,
}

impl EliminationDraft {
    fn new() -> Self {
        Self {
            field: FieldDraft::new(),
            double: RwSignal::new(false),
            bronze: RwSignal::new(false),
            plan: RwSignal::new(EliminationCreationDraft::default()),
        }
    }

    fn topology(self) -> Topology {
        if self.double.get() {
            Topology::Double
        } else {
            Topology::Single {
                bronze: self.bronze.get(),
            }
        }
    }

    fn configuration(self, clock: Clock, stages: &[Stage]) -> Result<Config, String> {
        let config = build_elimination_creation_configuration(
            &self.plan.get(),
            self.topology(),
            stages,
            clock,
        )
        .map_err(|error| error.to_string())?;
        validate_configuration(config)
    }
}

#[component]
pub(super) fn EliminationCreationForm() -> impl IntoView {
    let i18n = use_i18n();
    let common = CommonDraft::new(true, true);
    let draft = EliminationDraft::new();
    let params = Store::new(ChallengeParams::default());
    let clock = clock_signal(params);
    let topology = Signal::derive(move || draft.topology());
    let stages = Memo::new(move |_| {
        elimination_stages_for_field(topology.get(), draft.field.minimum.get() as usize)
    });
    let reveal_invalid = RwSignal::new(0_u32);
    Effect::new(move |previous: Option<TimeMode>| {
        let mode = params.time_signals().time_mode().get();
        if previous.is_some_and(|previous| previous != mode) {
            draft
                .plan
                .update(|plan| plan.normalize_time_mode(clock.get()));
        }
        mode
    });
    view! {
        <CreationShell
            draft=common
            format=Signal::derive(move || {
                if draft.double.get() {
                    Format::DoubleElimination
                } else {
                    Format::SingleElimination
                }
            })
            clock
            configuration=Callback::new(move |()| {
                draft.configuration(clock.get_untracked(), &stages.get_untracked())
            })
            field=Signal::derive(move || (
                Some(draft.field.maximum.get()),
                draft.field.minimum.get(),
            ))
            valid=Signal::derive(move || draft.field.valid(MAX_ELIMINATION_SEATS))
            participant_summary=Signal::derive(move || draft.field.summary())
            reveal_invalid_plan=reveal_invalid
        >
            <CreationDetails draft=common>
                <FieldSizeFields field=draft.field maximum=MAX_ELIMINATION_SEATS />
            </CreationDetails>
            <ClockFields params />
            <EliminationCreationEditor
                draft=draft.plan
                topology
                stages
                tournament_clock=clock
                bronze_match=draft.bronze
                reveal_invalid
            >
                <label class="flex flex-col gap-1.5 max-w-md">
                    <span class="ui-field-label">{t!(i18n, tournaments.creation.mode)}</span>
                    <select
                        class="ui-field-select"
                        name="Tournament Mode"
                        prop:value=move || {
                            if draft.double.get() {
                                "double_elimination"
                            } else {
                                "single_elimination"
                            }
                        }
                        on:change=move |event| {
                            draft.double.set(event_target_value(&event) == "double_elimination")
                        }
                    >
                        <option value="single_elimination">
                            {t!(i18n, tournaments.format.single_elimination)}
                        </option>
                        <option value="double_elimination">
                            {t!(i18n, tournaments.format.double_elimination)}
                        </option>
                    </select>
                </label>
            </EliminationCreationEditor>
        </CreationShell>
    }
}
