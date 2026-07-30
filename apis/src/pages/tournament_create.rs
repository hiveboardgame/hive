use crate::{
    common::{markdown_to_html, with_class, TimeParamsStoreFields, TournamentAction},
    components::{
        atoms::{
            date_time_picker::DateTimePicker,
            input_slider::InputSlider,
            rating::icon_for_speed,
            select_options::SelectOption,
            simple_switch::SimpleSwitch,
        },
        layouts::page_shell::PageShell,
        molecules::{panel::Panel, tiebreaker_picker::TiebreakerPicker},
        organisms::time_select::TimeSelect,
        update_from_event::{update_from_input, update_from_input_parsed},
    },
    functions::tournaments::tournament_name_taken,
    providers::{ApiRequestsProvider, AuthContext, ChallengeParams, ChallengeParamsStoreFields},
};
use chrono::{DateTime, Duration, Local, Utc};
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::use_navigate;
use reactive_stores::Store;
use shared_types::{
    CorrespondenceMode,
    GameSpeed,
    PointSystemDetails,
    PrettyString,
    ScoringMode,
    StartMode,
    Tiebreaker,
    TimeMode,
    TournamentDetails,
    TournamentMode,
};
use uuid::Uuid;

/// The game time controls an arena may use, as `(minutes, seconds)`.
///
/// Both numbers land in the linear part of `TimeParams`'s step scale, so they
/// double as the step indices. Short controls only: an arena is a sequence of
/// quick games against a clock, and a 30-minute game would fill the whole
/// event.
const ARENA_TIME_CONTROLS: [(i32, i32); 4] = [(1, 2), (3, 3), (5, 4), (10, 10)];

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
    /// Starts on schedule, and advances to the next round as soon as every game
    /// in the current one is finished — rather than waiting for the organizer to
    /// press anything. An arena needs it: it pairs continuously.
    pub fully_automated: RwSignal<bool>,
    /// Elimination only: play off third place instead of tying the beaten
    /// semi-finalists.
    pub third_place_match: RwSignal<bool>,
    /// Arena only, in minutes here — an arena is bounded by a clock rather than
    /// a round count, and minutes is what an organizer thinks in.
    pub arena_duration_minutes: RwSignal<i32>,
}

impl TournamentSignals {
    pub fn new() -> Self {
        Self {
            name: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            scoring: RwSignal::new(ScoringMode::Game),
            // Whatever the starting mode ranks by. `RawPoints` is deliberately
            // absent: it is the primary score, prepended by the engine, and
            // never a *tie* breaker.
            tiebreakers: RwSignal::new(
                Tiebreaker::defaults_for(TournamentMode::DoubleRoundRobin)
                    .into_iter()
                    .map(Some)
                    .collect(),
            ),
            seats: RwSignal::new(4),
            min_seats: RwSignal::new(4),
            rounds: RwSignal::new(1),
            invite_only: RwSignal::new(false),
            mode: RwSignal::new(TournamentMode::DoubleRoundRobin),
            series: RwSignal::new(None),
            starts_at: RwSignal::new(Utc::now()),
            round_duration: RwSignal::new(7),
            fully_automated: RwSignal::new(false),
            third_place_match: RwSignal::new(false),
            arena_duration_minutes: RwSignal::new(60),
        }
    }
}

impl Default for TournamentSignals {
    fn default() -> Self {
        Self::new()
    }
}

impl TournamentSignals {
    /// Puts every field back to its default.
    ///
    /// The signals live at the app root so the form survives being navigated
    /// away from and back — losing a half-filled tournament to a stray click is
    /// costly. Called once a tournament has actually been created, so the *next*
    /// one does not start out as a copy of it.
    pub fn reset(&self) {
        let fresh = Self::new();
        self.name.set(fresh.name.get_untracked());
        self.description.set(fresh.description.get_untracked());
        self.scoring.set(fresh.scoring.get_untracked());
        self.tiebreakers.set(fresh.tiebreakers.get_untracked());
        self.seats.set(fresh.seats.get_untracked());
        self.min_seats.set(fresh.min_seats.get_untracked());
        self.rounds.set(fresh.rounds.get_untracked());
        self.invite_only.set(fresh.invite_only.get_untracked());
        self.mode.set(fresh.mode.get_untracked());
        self.series.set(fresh.series.get_untracked());
        self.starts_at.set(Utc::now());
        self.round_duration
            .set(fresh.round_duration.get_untracked());
        self.fully_automated
            .set(fresh.fully_automated.get_untracked());
        self.third_place_match
            .set(fresh.third_place_match.get_untracked());
        self.arena_duration_minutes
            .set(fresh.arena_duration_minutes.get_untracked());
    }
}

pub fn provide_tournament_signals() {
    provide_context(TournamentSignals::default());
}

#[component]
pub fn TournamentCreate() -> impl IntoView {
    let tournament = expect_context::<TournamentSignals>();
    let params = Store::new(ChallengeParams::default());
    let min_rating = RwSignal::new(500);
    let max_rating = RwSignal::new(2500);
    let organizer_start = RwSignal::new(true);
    let fixed_round_duration = RwSignal::new(false);
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();
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
    // Names are unique in the database. Checked as it is typed, the way the
    // registration form checks usernames, so the first anyone hears about a
    // clash is not a constraint violation after clicking create.
    let name_taken = Action::new(|name: &String| {
        let name = name.clone();
        async move { tournament_name_taken(name).await }
    });
    Effect::new(move |_| {
        let name = tournament.name.get();
        if name.len() >= 4 {
            name_taken.dispatch(name);
        }
    });
    let name_is_taken = move || {
        name_taken
            .value()
            .get()
            .is_some_and(|taken| taken.unwrap_or(false))
    };

    // Every rule the server enforces, mirrored here so the form can point at
    // the offending field instead of round-tripping a 500.
    let name_too_short = move || tournament.name.get().len() < 4;
    let description_too_short = move || tournament.description.get().len() < 50;
    let too_few_players =
        move || !tournament.mode.get().is_arena() && tournament.min_seats.get() < 2;
    let too_many_rounds = move || {
        tournament.mode.get().rounds_are_chosen()
            && tournament.rounds.get() >= tournament.min_seats.get()
    };
    // Running automatically includes starting on schedule, so a manual start
    // would leave nothing to start it. The switch is hidden in that case, so
    // this only catches the order "manual start first, automatic second".
    let needs_a_start_time = move || tournament.fully_automated.get() && organizer_start.get();

    let disable_create = move || {
        name_too_short()
            || description_too_short()
            || name_is_taken()
            || too_few_players()
            || too_many_rounds()
            || needs_a_start_time()
    };

    // Whatever the server still refuses, verbatim — the field-level checks above
    // cannot anticipate everything.
    let create_error = RwSignal::new(Option::<String>::None);

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

        let mode = tournament.mode.get_untracked();
        let details = TournamentDetails {
            name: tournament.name.get_untracked(),
            description: tournament.description.get_untracked(),
            scoring: tournament.scoring.get_untracked(),
            tiebreakers: tournament.tiebreakers.get_untracked(),
            invitees: vec![],
            seats: tournament.seats.get_untracked(),
            min_seats: tournament.min_seats.get_untracked(),
            // Only Swiss lets the organizer pick; every other format's round
            // count falls out of the field size and is written back at start.
            rounds: if mode.rounds_are_chosen() {
                tournament.rounds.get_untracked()
            } else {
                0
            },
            // An arena is walk-in by definition.
            invite_only: !mode.is_arena() && tournament.invite_only.get_untracked(),
            mode: mode.to_string(),
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
            fully_automated: tournament.fully_automated.get_untracked(),
            // Only an elimination bracket has a third place to play off.
            third_place_match: mode.is_elimination()
                && tournament.third_place_match.get_untracked(),
            arena_duration_seconds: mode
                .is_arena()
                .then(|| tournament.arena_duration_minutes.get_untracked() * 60),
            // Left at the mode's own convention. Overriding individual point
            // values is an organizer's escape hatch, not a routine choice, so
            // it stays out of the create form.
            points: PointSystemDetails::default(),
        };
        if auth_context.user.with(|a| a.is_some()) {
            let api = api.get();
            let action = TournamentAction::Create(Box::new(details));
            api.tournament(action);
            // Cleared here rather than on arrival: navigating away mid-edit and
            // coming back has to keep what has been filled in, but the *next*
            // tournament must not start out as a copy of this one.
            tournament.reset();
            let navigate = use_navigate();
            navigate("/tournaments", Default::default());
        }
    };
    let on_value_change = Callback::new(move |t: TimeMode| {
        params.time_signals().time_mode().update(|v| *v = t);
    });
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
        // A round robin plays every pairing, so its game count grows with the
        // square of the field — that is what caps it, not the engine.
        TournamentMode::SingleRoundRobin
        | TournamentMode::DoubleRoundRobin
        | TournamentMode::QuadrupleRoundRobin
        | TournamentMode::SextupleRoundRobin => 16,
        // An arena is bounded by its clock rather than its field, so it can
        // take as many players as turn up.
        TournamentMode::Arena => 256,
        _ => 64,
    });
    let mode_is_swiss = Signal::derive(move || tournament.mode.get().is_swiss());
    // An arena has no minimum: it opens on its clock, pairs whoever is there,
    // and ends when the clock does. Every other format pairs a fixed field and
    // needs two — so this has to be put *back* on the way out of arena, or the
    // next mode is created with a minimum the server rejects.
    Effect::new(move |previous: Option<bool>| {
        let arena = tournament.mode.get().is_arena();
        if previous != Some(arena) {
            if arena {
                tournament.min_seats.set(1);
                tournament.seats.set(100);
            } else if tournament.min_seats.get() < 2 {
                tournament.min_seats.set(4);
                tournament.seats.set(tournament.seats.get().max(4));
            }
        }
        arena
    });
    // `fully_automated` by itself only drives `progress` once a tournament is
    // already under way — it has no bearing on *starting*. So "run
    // automatically" has to switch the start over to a schedule too, or nothing
    // would ever start it.
    Effect::new(move || {
        if tournament.fully_automated.get() {
            organizer_start.set(false);
        }
    });
    // A correspondence arena is a contradiction: the arena clock runs in
    // minutes while a correspondence game runs in days. Forcing the mode is
    // idempotent, so it can run on every pass; seeding a preset is not, and only
    // happens on the way *into* arena — otherwise remounting the page would
    // throw away a control the organizer had chosen.
    Effect::new(move |previous: Option<bool>| {
        let arena = tournament.mode.get().is_arena();
        if arena {
            params.time_signals().time_mode().set(TimeMode::RealTime);
            if previous == Some(false) {
                let (minutes, seconds) = ARENA_TIME_CONTROLS[2];
                params.time_signals().step_min().set(minutes);
                params.time_signals().step_sec().set(seconds);
            }
        }
        arena
    });
    // Everyone runs out of legal opponents once the rounds reach the size of
    // the field that can actually start, and `NewTournament::new` refuses it —
    // so the slider stops where validation would.
    let max_rounds = Signal::derive(move || (tournament.min_seats.get() - 1).clamp(1, 16));
    Effect::new(move || {
        let limit = max_rounds.get();
        if tournament.rounds.get() > limit {
            tournament.rounds.set(limit);
        }
    });
    let mode_is_elimination = Signal::derive(move || tournament.mode.get().is_elimination());
    let mode_is_arena = Signal::derive(move || tournament.mode.get().is_arena());
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
                        // Only once something has been typed — an empty field is
                        // not yet wrong, it is just empty.
                        <Show when=move || name_too_short() && !tournament.name.get().is_empty()>
                            <small class="ui-field-error">"At least 4 characters."</small>
                        </Show>
                        <Show when=name_is_taken>
                            <small class="ui-field-error">
                                "A tournament with that name already exists."
                            </small>
                        </Show>
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
                        // Counts up while it is too short, so it is clear how much
                        // further there is to go rather than only that it is wrong.
                        <Show
                            when=move || {
                                description_too_short() && !tournament.description.get().is_empty()
                            }
                            fallback=|| {
                                view! {
                                    <small class="ui-field-helper">
                                        "Descriptions need at least 50 characters."
                                    </small>
                                }
                            }
                        >
                            <small class="ui-field-error">
                                {move || {
                                    format!(
                                        "{} of 50 characters.",
                                        tournament.description.get().len(),
                                    )
                                }}
                            </small>
                        </Show>
                    </div>

                    <div class="grid gap-4 sm:grid-cols-2">
                        // An arena has no minimum — it opens on its clock and
                        // pairs whoever turned up.
                        <Show when=move || !mode_is_arena.get()>
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
                                <Show when=too_few_players>
                                    <small class="ui-field-error">
                                        "This format needs at least 2 players."
                                    </small>
                                </Show>
                            </div>
                        </Show>

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

                        <Show when=mode_is_swiss>
                            <div class="ui-setting-group">
                                <div class="flex gap-3 justify-between items-center">
                                    <span class="ui-field-label">"Rounds"</span>
                                    <span class="font-bold text-gray-900 dark:text-gray-100">
                                        {tournament.rounds}
                                    </span>
                                </div>
                                <InputSlider
                                    signal_to_update=tournament.rounds
                                    name="Rounds"
                                    min=1
                                    max=max_rounds
                                    step=1
                                />
                                <Show when=too_many_rounds>
                                    <small class="ui-field-error">
                                        "More rounds than players — lower the rounds or raise the minimum."
                                    </small>
                                </Show>
                                <small class="ui-field-helper">
                                    "A Swiss cannot pair more rounds than it has players."
                                </small>
                            </div>
                        </Show>
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
                                <SelectOption
                                    value=tournament.mode
                                    is="SingleRoundRobin"
                                    text=TournamentMode::SingleRoundRobin.pretty_string()
                                />
                                <SelectOption
                                    value=tournament.mode
                                    is="SingleElimination"
                                    text=TournamentMode::SingleElimination.pretty_string()
                                />
                                <SelectOption
                                    value=tournament.mode
                                    is="DoubleElimination"
                                    text=TournamentMode::DoubleElimination.pretty_string()
                                />
                                <SelectOption
                                    value=tournament.mode
                                    is="Arena"
                                    text=TournamentMode::Arena.pretty_string()
                                />
                                <SelectOption
                                    value=tournament.mode
                                    is="DoubleSwiss"
                                    text=TournamentMode::DoubleSwiss.pretty_string()
                                />
                                <SelectOption
                                    value=tournament.mode
                                    is="DutchSwiss"
                                    text=TournamentMode::DutchSwiss.pretty_string()
                                />
                                <SelectOption
                                    value=tournament.mode
                                    is="BursteinSwiss"
                                    text=TournamentMode::BursteinSwiss.pretty_string()
                                />
                            </select>
                        </label>

                        // An arena has no concept of a match: it scores each
                        // game, with streak and berserk bonuses on top, and
                        // ranks by its own fixed order. `ScoringMode` is not
                        // read for arenas at all, so offering it would be a
                        // control that does nothing.
                        <Show when=move || !mode_is_arena.get()>
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
                                    <SelectOption
                                        value=tournament.scoring
                                        is="Match"
                                        text=ScoringMode::Match.pretty_string()
                                    />
                                </select>
                            </label>
                        </Show>
                    </div>

                    <TiebreakerPicker
                        mode=Signal::derive(move || tournament.mode.get())
                        selected=tournament.tiebreakers
                    />

                    <div class="space-y-3 ui-setting-group">
                        // An arena's whole point is that anyone can walk in
                        // while it runs, so it is never invite only.
                        <Show when=move || !mode_is_arena.get()>
                            <div class="flex gap-3 items-center">
                                <SimpleSwitch checked=tournament.invite_only />
                                <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                    "Invite Only"
                                </span>
                            </div>
                        </Show>
                        <div class="flex flex-col gap-1">
                            <div class="flex gap-3 items-center">
                                <SimpleSwitch checked=tournament.fully_automated />
                                <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                    "Run automatically"
                                </span>
                            </div>
                            <small class="ui-field-helper">
                                "Starts at the scheduled time and moves to the next round as soon as every game in the round is finished."
                            </small>
                        </div>
                        <Show when=mode_is_elimination>
                            <div class="flex gap-3 items-center">
                                <SimpleSwitch checked=tournament.third_place_match />
                                <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                    "Play off third place"
                                </span>
                            </div>
                        </Show>
                        <Show when=mode_is_arena>
                            <div class="flex flex-col gap-1.5">
                                <div class="flex gap-3 justify-between items-center">
                                    <span class="ui-field-label">"Arena length (minutes)"</span>
                                    <span class="font-bold text-gray-900 dark:text-gray-100">
                                        {tournament.arena_duration_minutes}
                                    </span>
                                </div>
                                // Under an hour there is not enough time for a
                                // streak to mean anything; beyond five it stops
                                // being one sitting.
                                <InputSlider
                                    signal_to_update=tournament.arena_duration_minutes
                                    name="Arena Duration"
                                    min=60
                                    max=300
                                    step=15
                                />
                            </div>
                        </Show>
                        <div class="flex flex-col gap-3">
                            // Hidden rather than disabled when the tournament
                            // runs itself: a scheduled start is part of what
                            // "automatically" means, and offering a manual
                            // start alongside it only invites the combination
                            // where nothing ever starts.
                            <Show when=move || !tournament.fully_automated.get()>
                                <div class="flex gap-3 items-center">
                                    <SimpleSwitch checked=organizer_start />
                                    <span class="text-sm font-medium text-gray-900 dark:text-gray-100">
                                        "Manual start"
                                    </span>
                                </div>
                            </Show>
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
                        // An arena pairs continuously against a wall clock, so
                        // a correspondence game — days per move — cannot work in
                        // one at all. It gets a short list of real-time controls
                        // instead of the full picker.
                        <Show
                            when=move || mode_is_arena.get()
                            fallback=move || {
                                view! {
                                    <TimeSelect
                                        is_tournament=true
                                        params
                                        on_value_change
                                        allowed_values=vec![
                                            TimeMode::RealTime,
                                            TimeMode::Correspondence,
                                        ]
                                    />
                                }
                            }
                        >
                            <span class="ui-field-label">"Game time control"</span>
                            <div class="flex flex-wrap gap-2">
                                {ARENA_TIME_CONTROLS
                                    .iter()
                                    .map(|(minutes, seconds)| {
                                        let (minutes, seconds) = (*minutes, *seconds);
                                        let speed = GameSpeed::from_base_increment(
                                            Some(minutes * 60),
                                            Some(seconds),
                                        );
                                        let title = format!(
                                            "{speed}\n{minutes} min base time\n+{seconds} sec per move",
                                        );
                                        let selected = move || {
                                            params.time_signals().step_min().get() == minutes
                                                && params.time_signals().step_sec().get() == seconds
                                        };
                                        view! {
                                            <button
                                                type="button"
                                                title=title
                                                class=move || {
                                                    if selected() {
                                                        "flex gap-1 items-center ui-button ui-button-primary ui-button-sm"
                                                    } else {
                                                        "flex gap-1 items-center ui-button ui-button-secondary ui-button-sm"
                                                    }
                                                }
                                                on:click=move |_| {
                                                    params.time_signals().step_min().set(minutes);
                                                    params.time_signals().step_sec().set(seconds);
                                                }
                                            >
                                                <Icon icon=icon_for_speed(speed) attr:class="size-4" />
                                                {format!("{minutes}+{seconds}")}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </Show>
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

                    // An arena has no rounds at all — it is bounded by its own
                    // clock, which is set above.
                    <Show when=move || !mode_is_arena.get()>
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
                    </Show>
                </div>
            </div>

            // Says which field is wrong rather than only greying the button out,
            // so it is clear what to go and change.
            <div class="flex flex-col gap-1 items-end">
                <Show when=name_too_short>
                    <small class="ui-field-error">"Name needs at least 4 characters."</small>
                </Show>
                <Show when=name_is_taken>
                    <small class="ui-field-error">
                        "A tournament with that name already exists."
                    </small>
                </Show>
                <Show when=description_too_short>
                    <small class="ui-field-error">
                        "Description needs at least 50 characters."
                    </small>
                </Show>
                <Show when=too_few_players>
                    <small class="ui-field-error">
                        "This format needs at least 2 players — raise the minimum."
                    </small>
                </Show>
                <Show when=too_many_rounds>
                    <small class="ui-field-error">
                        "A Swiss needs more players than rounds — lower the rounds or raise the minimum."
                    </small>
                </Show>
                <Show when=needs_a_start_time>
                    <small class="ui-field-error">
                        "Running automatically needs a scheduled start — turn off manual start."
                    </small>
                </Show>
                <Show when=move || create_error.get().is_some()>
                    <small class="ui-field-error">{move || create_error.get()}</small>
                </Show>
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
