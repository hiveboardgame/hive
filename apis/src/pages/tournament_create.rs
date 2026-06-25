use crate::{
    common::{markdown_to_html, with_class, TimeParamsStoreFields, TournamentAction},
    components::{
        atoms::{
            date_time_picker::DateTimePicker,
            input_slider::InputSlider,
            select_options::SelectOption,
            simple_switch::SimpleSwitch,
        },
        layouts::page_shell::PageShell,
        molecules::panel::Panel,
        organisms::time_select::TimeSelect,
        update_from_event::{update_from_input, update_from_input_parsed},
    },
    providers::{ApiRequestsProvider, AuthContext, ChallengeParams, ChallengeParamsStoreFields},
};
use chrono::{DateTime, Duration, Local, Utc};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use reactive_stores::Store;
use shared_types::{
    CorrespondenceMode,
    PrettyString,
    ScoringMode,
    StartMode,
    Tiebreaker,
    TimeMode,
    TournamentDetails,
    TournamentMode,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct TournamentSignals {
    pub name: RwSignal<String>,
    pub description: RwSignal<String>,
    pub scoring: RwSignal<ScoringMode>,
    pub tiebreakers: RwSignal<Vec<Option<Tiebreaker>>>,
    pub seats: RwSignal<i32>,
    pub min_seats: RwSignal<i32>,
    pub rounds: RwSignal<i32>,
    pub invite_only: RwSignal<bool>,
    pub mode: RwSignal<TournamentMode>,
    pub series: RwSignal<Option<Uuid>>,
    pub starts_at: RwSignal<DateTime<Utc>>,
    pub round_duration: RwSignal<i32>,
}

impl TournamentSignals {
    pub fn new() -> Self {
        Self {
            name: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            scoring: RwSignal::new(ScoringMode::Game),
            tiebreakers: RwSignal::new(vec![
                Some(Tiebreaker::RawPoints),
                Some(Tiebreaker::HeadToHead),
                Some(Tiebreaker::WinsAsBlack),
                Some(Tiebreaker::SonnebornBerger),
            ]),
            seats: RwSignal::new(4),
            min_seats: RwSignal::new(4),
            rounds: RwSignal::new(1),
            invite_only: RwSignal::new(false),
            mode: RwSignal::new(TournamentMode::DoubleRoundRobin),
            series: RwSignal::new(None),
            starts_at: RwSignal::new(Utc::now()),
            round_duration: RwSignal::new(7),
        }
    }
}

impl Default for TournamentSignals {
    fn default() -> Self {
        Self::new()
    }
}

#[component]
pub fn TournamentCreate() -> impl IntoView {
    let tournament = TournamentSignals::default();
    let params = Store::new(ChallengeParams::default());
    let min_rating = RwSignal::new(500);
    let max_rating = RwSignal::new(2500);
    let organizer_start = RwSignal::new(true);
    let fixed_round_duration = RwSignal::new(false);
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();
    let account = auth_context.user;
    let user_allowed_to_run_swiss = Signal::derive(move || {
        account.with(|a| {
            if let Some(account) = a.as_ref() {
                account.user.admin || account.username == "stepanzo"
            } else {
                false
            }
        })
    });
    let rating_string = move || {
        format!(
            "Min Rating: {}/ Max Rating: {}",
            if min_rating() < 500 {
                "Any".to_owned()
            } else {
                min_rating.get().to_string()
            },
            if max_rating() > 2500 {
                "Any".to_owned()
            } else {
                max_rating().to_string()
            }
        )
    };
    let disable_create =
        move || tournament.name.get().len() < 4 || tournament.description.get().len() < 50;

    let create = move |_| {
        let time_mode = params.time_signals().time_mode().get_untracked();
        let (time_base, time_increment) = match time_mode {
            TimeMode::Untimed => (None, None),
            TimeMode::RealTime => (
                Some(params.time_signals().get().total_seconds()),
                Some(params.time_signals().get().sec_per_move()),
            ),
            TimeMode::Correspondence => match params.time_signals().corr_mode().get_untracked() {
                CorrespondenceMode::DaysPerMove => (
                    None,
                    Some(params.time_signals().corr_days().get_untracked() * 86400),
                ),
                CorrespondenceMode::TotalTimeEach => (
                    Some(params.time_signals().corr_days().get_untracked() * 86400),
                    None,
                ),
            },
        };

        let band_lower = if min_rating.get_untracked() < 500 {
            None
        } else {
            Some(min_rating.get_untracked())
        };
        let band_upper = if max_rating.get_untracked() > 2500 {
            None
        } else {
            Some(max_rating.get_untracked())
        };
        let round_duration =
            if fixed_round_duration.get_untracked() && time_mode == TimeMode::RealTime {
                Some(tournament.round_duration.get_untracked())
            } else {
                None
            };

        let details = TournamentDetails {
            name: tournament.name.get_untracked(),
            description: tournament.description.get_untracked(),
            scoring: tournament.scoring.get_untracked(),
            tiebreakers: tournament.tiebreakers.get_untracked(),
            invitees: vec![],
            seats: tournament.seats.get_untracked(),
            min_seats: tournament.min_seats.get_untracked(),
            rounds: tournament.rounds.get_untracked(),
            invite_only: tournament.invite_only.get_untracked(),
            mode: tournament.mode.get_untracked().to_string(),
            time_mode,
            time_base,
            time_increment,
            band_upper,
            band_lower,
            series: tournament.series.get_untracked(),
            start_mode: if organizer_start.get_untracked() {
                StartMode::Manual
            } else {
                StartMode::Date
            },
            starts_at: if organizer_start.get_untracked() {
                None
            } else {
                Some(tournament.starts_at.get_untracked())
            },
            round_duration,
        };
        if auth_context.user.with(|a| a.is_some()) {
            let api = api.get();
            let action = TournamentAction::Create(Box::new(details));
            api.tournament(action);
            let navigate = use_navigate();
            navigate("/tournaments", Default::default());
        }
    };
    let on_value_change = Callback::new(move |t: TimeMode| {
        params.time_signals().time_mode().update(|v| *v = t);
    });
    let allowed_values = vec![TimeMode::RealTime, TimeMode::Correspondence];
    let tournament_length = move || {
        if fixed_round_duration() {
            format!(
                "Maximum tournament duration {} days",
                tournament.rounds.get() * tournament.round_duration.get()
            )
        } else {
            String::from("Tournament length not automatically enforced")
        }
    };
    let is_not_preview_desc = RwSignal::new(true);
    let markdown_desc = move || markdown_to_html(&tournament.description.get());

    let max_seats = Signal::derive(move || match tournament.mode.get() {
        TournamentMode::DoubleSwiss => 64,
        _ => 16,
    });
    Effect::new(move || {
        let current_max = max_seats.get();
        if tournament.seats.get() > current_max {
            tournament.seats.set(current_max);
        }
    });
    view! {
        <PageShell>
            <div class="flex flex-col gap-1">
                <h1 class="ui-page-title">"Create Tournament"</h1>
                <p class="ui-page-subtitle">
                    "Set tournament details, entry limits, time controls, and start rules."
                </p>
            </div>

            <div class="grid gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(20rem,0.85fr)]">
                <Panel title="Tournament Details" body_class="space-y-5">
                    <label class="flex flex-col gap-1.5">
                        <span class="ui-field-label">"Tournament name"</span>
                        <input
                            class="ui-field-input"
                            name="Tournament name"
                            type="text"
                            prop:value=tournament.name
                            placeholder="At least a 4 character name"
                            on:input=update_from_input(tournament.name)
                            maxlength="50"
                        />
                    </label>

                    <div class="flex flex-col gap-1.5">
                        <div class="flex flex-wrap gap-2 justify-between items-center">
                            <span class="ui-field-label">"Description"</span>
                            <div class="flex flex-wrap gap-2">
                                <button
                                    type="button"
                                    on:click=move |_| is_not_preview_desc.update(|b| *b = !*b)
                                    class="py-1 px-3 text-xs ui-button ui-button-secondary ui-button-md"
                                >
                                    {move || if is_not_preview_desc() { "Preview" } else { "Edit" }}
                                </button>

                                <a
                                    class="py-1 px-3 text-xs ui-button ui-button-ghost ui-button-md no-link-style"
                                    href="https://commonmark.org/help/"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                >
                                    "Markdown"
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
                                prop:value=tournament.description
                                placeholder="At least a 50 character description. Markdown supported."
                                on:input=update_from_input(tournament.description)
                                maxlength="2000"
                            ></textarea>
                        </Show>
                        <small class="ui-field-helper">
                            "Descriptions need at least 50 characters."
                        </small>
                    </div>

                    <div class="grid gap-4 sm:grid-cols-2">
                        <div class="ui-setting-group">
                            <div class="flex gap-3 justify-between items-center">
                                <span class="ui-field-label">"Min players"</span>
                                <span class="font-bold text-gray-900 dark:text-gray-100">
                                    {tournament.min_seats}
                                </span>
                            </div>
                            <InputSlider
                                signal_to_update=tournament.min_seats
                                name="Seats"
                                min=2
                                max=tournament.seats
                                step=1
                            />
                        </div>

                        <div class="ui-setting-group">
                            <div class="flex gap-3 justify-between items-center">
                                <span class="ui-field-label">"Max players"</span>
                                <span class="font-bold text-gray-900 dark:text-gray-100">
                                    {tournament.seats}
                                </span>
                            </div>
                            <InputSlider
                                signal_to_update=tournament.seats
                                name="Min Seats"
                                min=tournament.min_seats
                                max=max_seats
                                step=1
                            />
                        </div>
                    </div>

                    <div class="grid gap-4 sm:grid-cols-2">
                        <label class="flex flex-col gap-1.5">
                            <span class="ui-field-label">"Mode"</span>
                            <select
                                class="ui-field-select"
                                name="Tournament Mode"
                                on:change=update_from_input_parsed(tournament.mode)
                            >
                                <SelectOption
                                    value=tournament.mode
                                    is="DoubleRoundRobin"
                                    text=TournamentMode::DoubleRoundRobin.pretty_string()
                                />
                                <SelectOption
                                    value=tournament.mode
                                    is="QuadrupleRoundRobin"
                                    text=TournamentMode::QuadrupleRoundRobin.pretty_string()
                                />
                                <SelectOption
                                    value=tournament.mode
                                    is="SextupleRoundRobin"
                                    text=TournamentMode::SextupleRoundRobin.pretty_string()
                                />
                                <Show when=user_allowed_to_run_swiss>
                                    <SelectOption
                                        value=tournament.mode
                                        is="DoubleSwiss"
                                        text=TournamentMode::DoubleSwiss.pretty_string()
                                    />
                                </Show>
                            </select>
                        </label>

                        <label class="flex flex-col gap-1.5">
                            <span class="ui-field-label">"Scoring"</span>
                            <select
                                class="ui-field-select"
                                name="Scoring Mode"
                                on:change=update_from_input_parsed(tournament.scoring)
                            >
                                <SelectOption
                                    value=tournament.scoring
                                    is="Game"
                                    text=ScoringMode::Game.pretty_string()
                                />
                            </select>
                        </label>
                    </div>

                    <div class="space-y-3 ui-setting-group">
                        <div class="flex gap-3 items-center">
                            <SimpleSwitch checked=tournament.invite_only />
                            <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                "Invite Only"
                            </span>
                        </div>
                        <div class="flex flex-col gap-3">
                            <div class="flex gap-3 items-center">
                                <SimpleSwitch checked=organizer_start />
                                <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                    "Manual start"
                                </span>
                            </div>
                            <Show when=move || !organizer_start()>
                                <DateTimePicker
                                    text="Choose a start time"
                                    min=Local::now()
                                    max=Local::now() + Duration::weeks(12)
                                    success_callback=Callback::from(move |local| {
                                        tournament
                                            .starts_at
                                            .update(|v| {
                                                *v = local;
                                            })
                                    })
                                    failure_callback=Callback::new(move |_| {
                                        organizer_start.set(true)
                                    })
                                />
                            </Show>
                        </div>
                    </div>
                </Panel>

                <div class="flex flex-col gap-6">
                    <Panel title="Time Controls" body_class="space-y-4">
                        <TimeSelect is_tournament=true params on_value_change allowed_values />
                    </Panel>

                    <Panel title="Rating Band" body_class="space-y-4">
                        <p class="ui-notice">{rating_string}</p>
                        <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-1 xl:grid-cols-2">
                            <div class="ui-setting-group">
                                <span class="ui-field-label">"Min rating"</span>
                                <InputSlider
                                    signal_to_update=min_rating
                                    name="Min rating"
                                    min=400
                                    max=Signal::derive(move || { max_rating() - 100 })
                                    step=100
                                />
                            </div>
                            <div class="ui-setting-group">
                                <span class="ui-field-label">"Max rating"</span>
                                <InputSlider
                                    signal_to_update=max_rating
                                    name="Max rating"
                                    min=Signal::derive(move || { min_rating() + 100 })
                                    max=2600
                                    step=100
                                />
                            </div>
                        </div>
                    </Panel>

                    <Panel title="Rounds" body_class="space-y-4">
                        <Show when=move || {
                            params.time_signals().time_mode().get() == TimeMode::RealTime
                        }>
                            <div class="space-y-3 ui-setting-group">
                                <div class="flex gap-3 items-center">
                                    <SimpleSwitch checked=fixed_round_duration />
                                    <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                        "Fixed round duration"
                                    </span>
                                </div>
                                <Show when=fixed_round_duration>
                                    <div>
                                        <div class="flex gap-3 justify-between items-center">
                                            <span class="ui-field-label">"Round duration"</span>
                                            <span class="font-bold text-gray-900 dark:text-gray-100">
                                                {tournament.round_duration} " days"
                                            </span>
                                        </div>
                                        <InputSlider
                                            signal_to_update=tournament.round_duration
                                            name="Round duration in days"
                                            min=1
                                            max=90
                                            step=1
                                        />
                                    </div>
                                </Show>
                            </div>
                        </Show>
                        <p class="ui-field-helper">{tournament_length}</p>
                    </Panel>
                </div>
            </div>

            <div class="flex justify-end">
                <button
                    class="w-full sm:w-auto ui-button ui-button-primary ui-button-md"
                    prop:disabled=disable_create
                    on:click=create
                >
                    "Create Tournament"
                </button>
            </div>
        </PageShell>
    }
}
