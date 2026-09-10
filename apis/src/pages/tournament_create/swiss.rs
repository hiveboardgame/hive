use super::common::{
    clock_signal,
    release_policy_from_token,
    release_policy_token,
    validate_configuration,
    ClockFields,
    CommonDraft,
    CreationDetails,
    CreationShell,
};
use crate::{
    components::molecules::{panel::Panel, tiebreaker_picker::TiebreakerPicker},
    i18n::*,
    providers::ChallengeParams,
};
use leptos::prelude::*;
use reactive_stores::Store;
use shared_types::tournament::{
    swiss::{
        swiss_creation_tiebreakers,
        BuchholzOptions as SwissBuchholzOptions,
        Config as SwissConfig,
        Criterion as SwissCriterion,
        PrimaryScore as DoubleSwissPrimaryScore,
        ProgressiveOptions as SwissProgressiveOptions,
        RoundConfiguration as SwissRoundConfiguration,
        ScoreBasis as SwissScoreBasis,
        SonnebornBergerOptions as SwissSonnebornBergerOptions,
    },
    BotAdmission,
    Clock,
    Config,
    Format,
    FormatConfig,
    ReleasePolicy,
    MAX_SWISS_SEATS,
    MIN_SWISS_START_SEATS,
};
use std::num::NonZeroU32;

#[derive(Clone, Copy)]
struct SwissDraft {
    capacity: RwSignal<String>,
    seats: RwSignal<i32>,
    extra_rounds: RwSignal<i32>,
    double: RwSignal<bool>,
    primary_score: RwSignal<DoubleSwissPrimaryScore>,
    release_policy: RwSignal<ReleasePolicy>,
    tiebreakers: RwSignal<Vec<SwissCriterion>>,
}

impl SwissDraft {
    fn new() -> Self {
        Self {
            capacity: RwSignal::new(String::from("32")),
            seats: RwSignal::new(32),
            extra_rounds: RwSignal::new(0),
            double: RwSignal::new(false),
            primary_score: RwSignal::new(DoubleSwissPrimaryScore::GamePoints),
            release_policy: RwSignal::new(ReleasePolicy::FullyUnlocked),
            tiebreakers: RwSignal::new(swiss_default_criteria()),
        }
    }

    fn capacity_valid(self) -> bool {
        self.capacity.with(|draft| {
            draft
                .parse::<i32>()
                .is_ok_and(|value| (MIN_SWISS_START_SEATS..=MAX_SWISS_SEATS).contains(&value))
        })
    }

    fn retain_tiebreakers(self) {
        let available = swiss_creation_tiebreakers(
            self.double
                .get_untracked()
                .then(|| self.primary_score.get_untracked()),
        );
        self.tiebreakers
            .update(|criteria| criteria.retain(|criterion| available.contains(criterion)));
    }

    fn set_double(self, double: bool) {
        batch(|| {
            self.double.set(double);
            self.retain_tiebreakers();
        });
    }

    fn set_primary_score(self, primary: DoubleSwissPrimaryScore) {
        batch(|| {
            self.primary_score.set(primary);
            self.retain_tiebreakers();
        });
    }

    fn set_capacity(self, capacity: String) {
        batch(|| {
            if let Ok(seats) = capacity.parse::<i32>() {
                if (MIN_SWISS_START_SEATS..=MAX_SWISS_SEATS).contains(&seats) {
                    self.seats.set(seats);
                    let rounds = |extra| {
                        SwissRoundConfiguration::automatic(extra)
                            .resolve(seats as usize)
                            .and_then(|rounds| rounds.resolved_rounds())
                    };
                    let mut extra = self.extra_rounds.get_untracked();
                    while extra != 0 && rounds(extra) == rounds(extra - extra.signum()) {
                        extra -= extra.signum();
                    }
                    self.extra_rounds.set(extra);
                }
            }
            self.capacity.set(capacity);
        });
    }

    fn configuration(self, clock: Clock) -> Result<Config, String> {
        if !self.capacity_valid() {
            return Err(String::from("Invalid Swiss capacity"));
        }
        let config = if self.double.get() {
            double_swiss_creation_configuration(
                self.extra_rounds.get(),
                self.primary_score.get(),
                self.release_policy.get(),
                &self.tiebreakers.get(),
                clock,
            )
        } else {
            let mut swiss = SwissConfig::automatic_swiss(self.extra_rounds.get(), clock);
            swiss.standings = swiss_standings(&self.tiebreakers.get());
            swiss_creation_configuration(swiss)
        };
        validate_configuration(config)
    }
}

fn swiss_default_criteria() -> Vec<SwissCriterion> {
    vec![
        SwissCriterion::Buchholz(SwissBuchholzOptions {
            cut_lowest: 0,
            cut_highest: 0,
            ..SwissBuchholzOptions::default()
        }),
        SwissCriterion::Buchholz(SwissBuchholzOptions {
            cut_lowest: 1,
            cut_highest: 0,
            ..SwissBuchholzOptions::default()
        }),
        SwissCriterion::SonnebornBerger(SwissSonnebornBergerOptions {
            score_basis: SwissScoreBasis::Primary,
            cut_lowest: 0,
            ..SwissSonnebornBergerOptions::default()
        }),
        SwissCriterion::Progressive(SwissProgressiveOptions {
            score_basis: SwissScoreBasis::Primary,
            cut_first_round: false,
        }),
    ]
}

fn swiss_standings(tiebreakers: &[SwissCriterion]) -> Vec<SwissCriterion> {
    std::iter::once(SwissCriterion::PrimaryScore)
        .chain(tiebreakers.iter().copied())
        .collect()
}

fn swiss_creation_configuration(swiss: SwissConfig) -> Config {
    Config {
        bot_admission: BotAdmission::HumansAndBots,
        format: FormatConfig::Swiss(swiss),
    }
}

fn double_swiss_creation_configuration(
    extra_rounds: i32,
    primary_score: DoubleSwissPrimaryScore,
    release_policy: ReleasePolicy,
    tiebreakers: &[SwissCriterion],
    clock: Clock,
) -> Config {
    let mut swiss = SwissConfig::automatic_double_swiss(extra_rounds, clock, primary_score);
    swiss.double_swiss_release_policy = release_policy;
    swiss.standings = swiss_standings(tiebreakers);
    swiss_creation_configuration(swiss)
}

#[component]
pub(super) fn SwissCreationForm() -> impl IntoView {
    let i18n = use_i18n();
    let common = CommonDraft::new(true, true);
    let draft = SwissDraft::new();
    let params = Store::new(ChallengeParams::default());
    let clock = clock_signal(params);
    let capacity_valid = Signal::derive(move || draft.capacity_valid());
    let swiss_preview_rounds = move |extra| {
        SwissRoundConfiguration::automatic(extra)
            .resolve(draft.seats.get() as usize)
            .and_then(|rounds| rounds.resolved_rounds())
            .map_or(0, NonZeroU32::get)
    };
    let swiss_adjustment_available = move |extra| {
        extra == 0 || swiss_preview_rounds(extra) != swiss_preview_rounds(extra - extra.signum())
    };
    let swiss_available_tiebreakers = Signal::derive(move || {
        swiss_creation_tiebreakers(draft.double.get().then(|| draft.primary_score.get()))
    });
    view! {
        <CreationShell
            draft=common
            format=Signal::derive(move || {
                if draft.double.get() { Format::DoubleSwiss } else { Format::Swiss }
            })
            clock
            configuration=Callback::new(move |()| draft.configuration(clock.get_untracked()))
            field=Signal::derive(move || (Some(draft.seats.get()), MIN_SWISS_START_SEATS))
            valid=capacity_valid
            // TODO: i18n once copy is approved.
            participant_summary=Signal::derive(move || {
                if capacity_valid.get() {
                    format!("Up to {} players", draft.seats.get())
                } else {
                    String::new()
                }
            })
        >
            <CreationDetails draft=common>

                <label class="flex flex-col gap-1.5 sm:col-span-2 ui-setting-group">
                    // TODO: i18n once copy is approved.
                    <span class="ui-field-label">"Maximum players"</span>
                    <input
                        class="ui-field-input"
                        name="Maximum players"
                        type="number"
                        min=MIN_SWISS_START_SEATS
                        max=MAX_SWISS_SEATS
                        prop:value=draft.capacity
                        on:input=move |event| draft.set_capacity(event_target_value(&event))
                    />
                    // TODO: i18n once copy is approved.
                    <small class="ui-field-helper">
                        "At least five players are needed to start."
                    </small>
                    <Show when=move || !capacity_valid.get()>
                        // TODO: i18n once copy is approved.
                        <small class="ui-field-error">"Choose between 5 and 160 players."</small>
                    </Show>
                </label>
            </CreationDetails>
            <ClockFields params />
            // TODO: i18n once copy is approved.
            <Panel title="Tournament rules" body_class="space-y-5">
                <label class="flex flex-col gap-1.5 max-w-md">
                    <span class="ui-field-label">{t!(i18n, tournaments.creation.mode)}</span>
                    <select
                        class="ui-field-select"
                        name="Tournament Mode"
                        prop:value=move || if draft.double.get() { "double_swiss" } else { "swiss" }
                        on:change=move |event| {
                            draft.set_double(event_target_value(&event) == "double_swiss")
                        }
                    >
                        <option value="swiss">{t!(i18n, tournaments.format.swiss)}</option>
                        <option value="double_swiss">
                            {t!(i18n, tournaments.format.double_swiss)}
                        </option>
                    </select>
                </label>
                <div class="space-y-3">
                    // TODO: i18n once copy is approved.
                    <span class="ui-field-label">"Rounds"</span>
                    <div class="grid gap-1 grid-cols-[1fr_1fr_1.7fr_1fr_1fr_1fr]">
                        // TODO: i18n once copy is approved.
                        {[
                            (-2, "−2"),
                            (-1, "−1"),
                            (0, "Optimal"),
                            (1, "+1"),
                            (2, "+2"),
                            (3, "+3"),
                        ]
                            .into_iter()
                            .map(|(extra, label)| {
                                view! {
                                    <button
                                        type="button"
                                        class=move || {
                                            if draft.extra_rounds.get() == extra {
                                                "px-2 ui-button ui-button-primary ui-button-md"
                                            } else {
                                                "px-2 ui-button ui-button-secondary ui-button-md"
                                            }
                                        }
                                        prop:disabled=move || {
                                            !capacity_valid.get() || !swiss_adjustment_available(extra)
                                        }
                                        on:click=move |_| draft.extra_rounds.set(extra)
                                    >
                                        {label}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    <Show when=capacity_valid>
                        // TODO: i18n once copy is approved.
                        <p class="ui-field-helper">
                            {move || {
                                format!(
                                    "{} rounds with {} players; adjusted to the number who start.",
                                    swiss_preview_rounds(draft.extra_rounds.get()),
                                    draft.seats.get(),
                                )
                            }}
                        </p>
                    </Show>
                </div>

                <div class="space-y-4">
                    <Show when=draft.double>
                        <div class="grid gap-4 sm:grid-cols-2">
                            <label class="flex flex-col gap-1.5 ui-setting-group">
                                // TODO: i18n once copy is approved.
                                <span class="ui-field-label">"Scoring"</span>
                                <select
                                    class="ui-field-select"
                                    prop:value=move || match draft.primary_score.get() {
                                        DoubleSwissPrimaryScore::GamePoints => "game_points",
                                        DoubleSwissPrimaryScore::MatchPoints => "match_points",
                                    }
                                    on:change=move |event| {
                                        let primary_score = match event_target_value(&event)
                                            .as_str()
                                        {
                                            "match_points" => DoubleSwissPrimaryScore::MatchPoints,
                                            _ => DoubleSwissPrimaryScore::GamePoints,
                                        };
                                        draft.set_primary_score(primary_score);
                                    }
                                >
                                    // TODO: i18n once copy is approved.
                                    <option value="game_points">"Game points"</option>
                                    // TODO: i18n once copy is approved.
                                    <option value="match_points">"Match points"</option>
                                </select>
                            </label>
                            <label class="flex flex-col gap-1.5 ui-setting-group">
                                // TODO: i18n once copy is approved.
                                <span class="ui-field-label">"Game availability"</span>
                                <select
                                    class="ui-field-select"
                                    prop:value=move || {
                                        release_policy_token(draft.release_policy.get())
                                    }
                                    on:change=move |event| {
                                        if let Some(policy) = release_policy_from_token(
                                            &event_target_value(&event),
                                        ) {
                                            draft.release_policy.set(policy);
                                        }
                                    }
                                >
                                    // TODO: i18n once copy is approved.
                                    <option value="fully_unlocked">
                                        "Release both games together"
                                    </option>
                                    // TODO: i18n once copy is approved.
                                    <option value="sequential_per_matchup">
                                        "Release Game 2 after Game 1"
                                    </option>
                                </select>
                            </label>
                        </div>
                    </Show>
                    <TiebreakerPicker
                        selected=draft.tiebreakers
                        available=swiss_available_tiebreakers
                    />
                </div>
            </Panel>
        </CreationShell>
    }
}
