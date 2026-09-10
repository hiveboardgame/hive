use crate::{
    common::{
        format_tournament_datetime,
        markdown_to_html,
        tournament_format_label,
        with_class,
        TimeParams,
        TournamentAction,
    },
    components::{
        atoms::{
            date_time_picker::DateTimePicker,
            input_slider::InputSlider,
            simple_switch::SimpleSwitch,
        },
        layouts::page_shell::PageShell,
        molecules::{panel::Panel, time_row::TimeRow},
        organisms::{time_select::TimeSelect, tournament_explanations::TournamentFormatFaq},
        update_from_event::update_from_input,
    },
    i18n::*,
    providers::{ApiRequestsProvider, ChallengeParams, ChallengeParamsStoreFields},
};
use chrono::{DateTime, Duration, Local, Utc};
use leptos::{leptos_dom::helpers::set_timeout_with_handle, prelude::*};
use reactive_stores::Store;
use shared_types::{
    tournament::{Clock, Config, Format, ReleasePolicy},
    TimeMode,
    TournamentDetails,
};
use std::time::Duration as StdDuration;

#[derive(Clone, Copy)]
pub(super) struct MetadataDraft {
    pub name: RwSignal<String>,
    pub description: RwSignal<String>,
}

#[derive(Clone, Copy)]
pub(super) struct AdmissionDraft {
    pub min_rating: RwSignal<i32>,
    pub max_rating: RwSignal<i32>,
    pub invite_only: Option<RwSignal<bool>>,
}

#[derive(Clone, Copy)]
pub(super) struct ScheduleDraft {
    pub allow_manual: bool,
    pub manual: RwSignal<bool>,
    pub starts_at: RwSignal<DateTime<Utc>>,
    pub valid: RwSignal<bool>,
}

#[derive(Clone, Copy)]
pub(super) struct CommonDraft {
    pub metadata: MetadataDraft,
    pub admission: AdmissionDraft,
    pub schedule: ScheduleDraft,
}

impl CommonDraft {
    pub fn new(allow_manual: bool, allow_invitations: bool) -> Self {
        Self {
            metadata: MetadataDraft {
                name: RwSignal::new(String::new()),
                description: RwSignal::new(String::new()),
            },
            admission: AdmissionDraft {
                min_rating: RwSignal::new(400),
                max_rating: RwSignal::new(2600),
                invite_only: allow_invitations.then(|| RwSignal::new(false)),
            },
            schedule: ScheduleDraft {
                allow_manual,
                manual: RwSignal::new(allow_manual),
                starts_at: RwSignal::new(Utc::now() + Duration::hours(1)),
                valid: RwSignal::new(true),
            },
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct FieldDraft {
    pub minimum: RwSignal<i32>,
    pub maximum: RwSignal<i32>,
}

impl FieldDraft {
    pub fn new() -> Self {
        Self {
            minimum: RwSignal::new(4),
            maximum: RwSignal::new(4),
        }
    }

    pub fn valid(self, maximum: i32) -> bool {
        (2..=maximum).contains(&self.maximum.get())
            && (2..=self.maximum.get()).contains(&self.minimum.get())
    }

    pub fn summary(self) -> String {
        // TODO: i18n once copy is approved.
        if self.minimum.get() == self.maximum.get() {
            format!("{} players", self.maximum.get())
        } else {
            format!("{}–{} players", self.minimum.get(), self.maximum.get())
        }
    }
}

pub(super) fn selected_clock(params: &TimeParams) -> Clock {
    Clock::from_time_parts(params.time_mode, params.base(), params.increment())
        .expect("bounded time controls produce valid clock parts")
        .expect("creation only offers timed games")
}

pub(super) fn clock_signal(params: Store<ChallengeParams>) -> Signal<Clock> {
    Signal::derive(move || selected_clock(&params.time_signals().get()))
}

pub(super) fn validate_configuration(configuration: Config) -> Result<Config, String> {
    configuration
        .validate_for_creation()
        .map_err(|error| error.to_string())?;
    Ok(configuration)
}

pub(super) fn release_policy_token(policy: ReleasePolicy) -> &'static str {
    match policy {
        ReleasePolicy::FullyUnlocked => "fully_unlocked",
        ReleasePolicy::SequentialPerMatchup => "sequential_per_matchup",
    }
}

pub(super) fn release_policy_from_token(token: &str) -> Option<ReleasePolicy> {
    match token {
        "fully_unlocked" => Some(ReleasePolicy::FullyUnlocked),
        "sequential_per_matchup" => Some(ReleasePolicy::SequentialPerMatchup),
        _ => None,
    }
}

#[component]
pub(super) fn CreationDetails(draft: CommonDraft, children: Children) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <Panel
            title=move || t_string!(i18n, tournaments.creation.details_title)
            body_class="space-y-5"
        >
            <MetadataFields draft=draft.metadata />
            {children()}
            <AdmissionFields draft=draft.admission />
            <SchedulingFields draft=draft.schedule />
        </Panel>
    }
}

#[component]
fn MetadataFields(draft: MetadataDraft) -> impl IntoView {
    let i18n = use_i18n();
    let name_touched = RwSignal::new(false);
    let name_too_short = move || draft.name.get().chars().count() < 4;
    let description_open = RwSignal::new(false);
    let is_not_preview_desc = RwSignal::new(true);
    let markdown_desc = move || markdown_to_html(&draft.description.get());
    view! {
        <label class="flex flex-col gap-1.5">
            <span class="ui-field-label">{t!(i18n, tournaments.creation.name)}</span>
            <input
                class="ui-field-input"
                name="Tournament name"
                type="text"
                prop:value=draft.name
                placeholder=move || { t_string!(i18n, tournaments.creation.name_placeholder) }
                on:input=update_from_input(draft.name)
                on:blur=move |_| name_touched.set(true)
                maxlength="50"
            />
            <Show when=move || name_touched.get() && name_too_short()>
                <small class="ui-field-error">
                    {t!(i18n, tournaments.creation.name_too_short)}
                </small>
            </Show>
        </label>
        <Show when=move || !description_open.get()>
            // TODO: i18n once copy is approved.
            <button
                type="button"
                class="ui-button ui-button-ghost ui-button-sm"
                on:click=move |_| description_open.set(true)
            >
                "+ Add description"
            </button>
        </Show>
        <Show when=description_open>
            <div class="flex flex-col gap-1.5">
                <div class="flex flex-wrap gap-2 justify-between items-center">
                    <span class="ui-field-label">{t!(i18n, tournaments.creation.description)}</span>
                    <div class="flex flex-wrap gap-2">
                        <button
                            type="button"
                            on:click=move |_| is_not_preview_desc.update(|b| *b = !*b)
                            class="py-1 px-3 text-xs ui-button ui-button-secondary ui-button-md"
                        >
                            {move || {
                                if is_not_preview_desc() {
                                    t_string!(i18n, tournaments.detail.preview).to_string()
                                } else {
                                    t_string!(i18n, tournaments.detail.edit).to_string()
                                }
                            }}
                        </button>

                        <a
                            class="py-1 px-3 text-xs ui-button ui-button-ghost ui-button-md no-link-style"
                            href="https://commonmark.org/help/"
                            target="_blank"
                            rel="noopener noreferrer"
                        >
                            {t!(i18n, tournaments.detail.markdown)}
                        </a>
                    </div>
                </div>
                <Show
                    when=is_not_preview_desc
                    fallback=move || {
                        view! {
                            <div
                                class=with_class(
                                    "ui-setting-group",
                                    "min-h-40 w-full break-words prose dark:prose-invert max-w-none",
                                )
                                inner_html=markdown_desc
                            />
                        }
                    }
                >
                    <textarea
                        class="ui-field-textarea min-h-40"
                        name="Tournament description"
                        prop:value=draft.description
                        placeholder=move || {
                            t_string!(
                                i18n,
                                tournaments.creation.description_placeholder
                            )
                        }
                        on:input=update_from_input(draft.description)
                        maxlength="2000"
                    ></textarea>
                </Show>
            </div>
        </Show>
    }
}

#[component]
fn AdmissionFields(draft: AdmissionDraft) -> impl IntoView {
    let i18n = use_i18n();
    let min_rating = draft.min_rating;
    let max_rating = draft.max_rating;
    let rating_string = move || {
        let minimum = if min_rating() < 500 {
            t_string!(i18n, tournaments.creation.any_rating).to_string()
        } else {
            min_rating.get().to_string()
        };
        let maximum = if max_rating() > 2500 {
            t_string!(i18n, tournaments.creation.any_rating).to_string()
        } else {
            max_rating().to_string()
        };
        t_string!(
            i18n,
            tournaments.creation.rating_summary,
            minimum = minimum,
            maximum = maximum,
        )
        .to_string()
    };
    let rating_summary = move || {
        let lower = min_rating.get();
        let upper = max_rating.get();
        // TODO: i18n once copy is approved.
        if lower < 500 && upper > 2500 {
            String::from("any rating")
        } else if lower < 500 {
            format!("rating up to {upper}")
        } else if upper > 2500 {
            format!("rating {lower} and above")
        } else {
            format!("rating {lower}–{upper}")
        }
    };
    view! {
        {draft
            .invite_only
            .map(|invite_only| {
                view! {
                    <div class="flex gap-3 items-center">
                        <SimpleSwitch checked=invite_only />
                        <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                            {t!(i18n, tournaments.creation.invite_only)}
                        </span>
                    </div>
                }
            })}
        <details>
            // TODO: i18n once copy is approved.
            <summary class="text-sm font-medium cursor-pointer">
                {move || format!("Rating limits · {}", rating_summary())}
            </summary>
            <div class="mt-3 space-y-3">
                <p class="ui-notice">{rating_string}</p>
                <div class="grid gap-4 sm:grid-cols-2">
                    <div class="ui-setting-group">
                        <span class="ui-field-label">
                            {t!(i18n, tournaments.creation.min_rating)}
                        </span>
                        <InputSlider
                            signal_to_update=min_rating
                            name="Min rating"
                            min=400
                            max=Signal::derive(move || { max_rating() - 100 })
                            step=100
                        />
                    </div>
                    <div class="ui-setting-group">
                        <span class="ui-field-label">
                            {t!(i18n, tournaments.creation.max_rating)}
                        </span>
                        <InputSlider
                            signal_to_update=max_rating
                            name="Max rating"
                            min=Signal::derive(move || { min_rating() + 100 })
                            max=2600
                            step=100
                        />
                    </div>
                </div>
            </div>
        </details>
    }
}

#[component]
fn SchedulingFields(draft: ScheduleDraft) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="space-y-3 ui-setting-group">
            <div class="flex flex-col gap-3">
                <Show when=move || draft.allow_manual>
                    <fieldset class="space-y-2">
                        // TODO: i18n once copy is approved.
                        <legend class="ui-field-label">"Start mode"</legend>
                        <div class="grid gap-2 sm:grid-cols-2">
                            <label class="flex gap-2 items-center text-sm">
                                <input
                                    type="radio"
                                    name="start-mode"
                                    prop:checked=draft.manual
                                    on:change=move |_| draft.manual.set(true)
                                />
                                // TODO: i18n once copy is approved.
                                "Start manually"
                            </label>
                            <label class="flex gap-2 items-center text-sm">
                                <input
                                    type="radio"
                                    name="start-mode"
                                    prop:checked=move || !draft.manual.get()
                                    on:change=move |_| draft.manual.set(false)
                                />
                                // TODO: i18n once copy is approved.
                                "Start at a scheduled time"
                            </label>
                        </div>
                    </fieldset>
                </Show>
                <Show when=move || { !draft.allow_manual || !draft.manual.get() }>
                    <DateTimePicker
                        input_id="tournament-start-time".to_string()
                        draft_valid=draft.valid
                        text=move || { t_string!(i18n, tournaments.creation.choose_start) }
                        min=Local::now()
                        max=Local::now() + Duration::weeks(12)
                        value=draft.starts_at.get_untracked().with_timezone(&Local)
                        success_callback=Callback::from(move |local| {
                            draft
                                .starts_at
                                .update(|v| {
                                    *v = local;
                                })
                        })
                    />
                </Show>
            </div>
        </div>
    }
}

#[component]
pub(super) fn FieldSizeFields(field: FieldDraft, maximum: i32) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="grid gap-4 sm:grid-cols-2">

            <div class="ui-setting-group">
                <div class="flex gap-3 justify-between items-center">
                    <span class="ui-field-label">{t!(i18n, tournaments.creation.min_players)}</span>
                    <span class="font-bold text-gray-900 dark:text-gray-100">{field.minimum}</span>
                </div>
                <InputSlider
                    signal_to_update=field.minimum
                    name="Seats"
                    min=2
                    max=field.maximum
                    step=1
                />
            </div>

            <div class="ui-setting-group">
                <div class="flex gap-3 justify-between items-center">
                    <span class="ui-field-label">{t!(i18n, tournaments.creation.max_players)}</span>
                    <span class="font-bold text-gray-900 dark:text-gray-100">{field.maximum}</span>
                </div>
                <InputSlider
                    signal_to_update=field.maximum
                    name="Min Seats"
                    min=field.minimum
                    max=maximum
                    step=1
                />
            </div>
        </div>
    }
}

#[component]
pub(super) fn ClockFields(params: Store<ChallengeParams>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <Panel
            title=move || t_string!(i18n, tournaments.creation.time_controls_title)
            body_class="space-y-4"
        >
            <TimeSelect
                is_tournament=true
                params
                allowed_values=vec![TimeMode::RealTime, TimeMode::Correspondence]
            />
        </Panel>
    }
}

#[component]
pub(super) fn CreationShell(
    draft: CommonDraft,
    format: Signal<Format>,
    clock: Signal<Clock>,
    configuration: Callback<(), Result<Config, String>>,
    field: Signal<(Option<i32>, i32)>,
    valid: Signal<bool>,
    participant_summary: Signal<String>,
    #[prop(default = RwSignal::new(0))] reveal_invalid_plan: RwSignal<u32>,
    children: Children,
) -> impl IntoView {
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let create_pending = ArcRwSignal::new(false);
    let pending_for_disabled = create_pending.clone();
    let can_create = Signal::derive(move || {
        valid.get()
            && draft.metadata.name.get().chars().count() >= 4
            && (draft.schedule.manual.get() || draft.schedule.valid.get())
    });
    let disable_create = Signal::derive(move || pending_for_disabled.get() || !can_create.get());
    let create_error = RwSignal::new(false);
    let create = Callback::new(move |_| {
        if create_pending.get_untracked() || !can_create.get_untracked() {
            return;
        }
        create_error.set(false);
        let Ok(configuration) = configuration.run(()) else {
            create_error.set(true);
            reveal_invalid_plan.update(|value| *value += 1);
            return;
        };
        let (seats, min_seats) = field.get_untracked();
        let minimum = draft.admission.min_rating.get_untracked();
        let maximum = draft.admission.max_rating.get_untracked();
        let details = TournamentDetails {
            name: draft.metadata.name.get_untracked(),
            description: Some(draft.metadata.description.get_untracked()),
            seats,
            min_seats,
            invite_only: draft
                .admission
                .invite_only
                .is_some_and(|value| value.get_untracked()),
            band_lower: (minimum >= 500).then_some(minimum),
            band_upper: (maximum <= 2500).then_some(maximum),
            starts_at: (!draft.schedule.manual.get_untracked())
                .then(|| draft.schedule.starts_at.get_untracked()),
            configuration,
        };
        create_pending.set(true);
        api.get()
            .tournament(TournamentAction::Create(Box::new(details)));
        let pending = create_pending.clone();
        let _ =
            set_timeout_with_handle(move || pending.set(false), StdDuration::from_millis(1_500));
    });
    view! {
        <PageShell>
            <div class="mx-auto space-y-5 w-full max-w-3xl sm:space-y-6">
                <div>
                    <h1 class="ui-page-title">{t!(i18n, tournaments.creation.title)}</h1>
                    <p class="ui-page-subtitle">
                        {move || tournament_format_label(i18n, format.get())}
                    </p>
                </div>
                {children()}
                <div class="space-y-4">
                    <div class="flex flex-wrap gap-y-2 gap-x-3 text-sm text-gray-600 dark:text-gray-300">
                        <span>{move || tournament_format_label(i18n, format.get())}</span>
                        // TODO: i18n once copy is approved.
                        <span>{participant_summary}</span>
                        <TimeRow time_control=Signal::derive(move || Some(clock.get())) />
                        // TODO: i18n once copy is approved.
                        <span>
                            {move || {
                                if draft.schedule.allow_manual && draft.schedule.manual.get() {
                                    String::from("Manual start")
                                } else if draft.schedule.valid.get() {
                                    format_tournament_datetime(draft.schedule.starts_at.get())
                                } else {
                                    String::from("Choose a valid start time")
                                }
                            }}
                        </span>
                    </div>
                    <Show when=move || create_error.get()>
                        <button
                            type="button"
                            class="underline ui-field-error"
                            on:click=move |_| reveal_invalid_plan.update(|value| *value += 1)
                        >
                            {t!(i18n, tournaments.creation.invalid_configuration)}
                        </button>
                    </Show>
                    <div class="flex justify-end">
                        <button
                            class="w-full sm:w-auto ui-button ui-button-primary ui-button-md"
                            prop:disabled=disable_create
                            on:click=move |event| create.run(event)
                        >
                            {t!(i18n, tournaments.creation.create)}
                        </button>
                    </div>
                </div>
                <details class="ui-panel">
                    // TODO: i18n once copy is approved.
                    <summary class="p-4 font-semibold cursor-pointer">
                        {move || {
                            format!("About {}", tournament_format_label(i18n, format.get()))
                        }}
                    </summary>
                    <div class="p-4 border-t border-gray-200 dark:border-gray-700">
                        <TournamentFormatFaq format=format />
                    </div>
                </details>
            </div>
        </PageShell>
    }
}
