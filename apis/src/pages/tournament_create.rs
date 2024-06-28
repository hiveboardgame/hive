use crate::common::{TimeSignals, TournamentAction};
use crate::components::organisms::time_select::TimeSelect;
use crate::components::update_from_event::{update_from_input, update_from_input_parsed};
use crate::{
    components::atoms::{input_slider::InputSlider, select_options::SelectOption},
    providers::{ApiRequests, AuthContext},
};
use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use leptos::ev::Event;
use leptos::*;
use leptos_router::use_navigate;
use shared_types::{CorrespondenceMode, TimeMode, TournamentDetails};
use uuid::Uuid;

const BUTTON_STYLE: &str = "flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95";

#[derive(Debug, Clone, Copy)]
pub struct TournamentTournamentSignals {
    pub name: RwSignal<String>,
    pub description: RwSignal<String>,
    pub scoring: RwSignal<String>,
    pub tiebreaker: RwSignal<Vec<Option<String>>>,
    pub seats: RwSignal<i32>,
    pub rounds: RwSignal<i32>,
    pub joinable: RwSignal<bool>,
    pub invite_only: RwSignal<bool>,
    pub mode: RwSignal<String>,
    pub time_mode: RwSignal<TimeMode>,
    pub time_base: StoredValue<Option<i32>>,
    pub time_increment: StoredValue<Option<i32>>,
    pub band_upper: RwSignal<Option<i32>>,
    pub band_lower: RwSignal<Option<i32>>,
    pub series: RwSignal<Option<Uuid>>,
    pub start_at: RwSignal<DateTime<Utc>>,
}

impl TournamentTournamentSignals {
    pub fn new() -> Self {
        Self {
            name: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            scoring: RwSignal::new(String::from("Match")),
            tiebreaker: RwSignal::new(vec![Some(String::from("Buchholz"))]),
            seats: RwSignal::new(4),
            rounds: RwSignal::new(2),
            joinable: RwSignal::new(true),
            invite_only: RwSignal::new(false),
            // TODO make this into a type
            mode: RwSignal::new(String::from("Double Round Robin")),
            time_mode: RwSignal::new(TimeMode::RealTime),
            time_base: store_value(Some(60)),
            time_increment: store_value(Some(0)),
            band_upper: RwSignal::new(None),
            band_lower: RwSignal::new(None),
            series: RwSignal::new(None),
            start_at: RwSignal::new(Utc::now()),
        }
    }
}

impl Default for TournamentTournamentSignals {
    fn default() -> Self {
        Self::new()
    }
}

#[component]
pub fn TournamentCreate() -> impl IntoView {
    let tournament = TournamentTournamentSignals::default();
    let time_signals = TimeSignals::default();
    let min_rating = RwSignal::new(500);
    let max_rating = RwSignal::new(2500);
    let organizer_start = RwSignal::new(true);
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

    let create = move |_| {
        let auth_context = expect_context::<AuthContext>();
        let account = match (auth_context.user)() {
            Some(Ok(Some(account))) => Some(account),
            _ => None,
        };
        tournament
            .time_mode
            .update_untracked(|v| *v = time_signals.time_control.get_untracked());
        match (tournament.time_mode)() {
            TimeMode::Untimed => {
                tournament.time_base.update_value(|v| *v = None);
                tournament.time_increment.update_value(|v| *v = None);
            }
            TimeMode::RealTime => {
                tournament
                    .time_base
                    .update_value(|v| *v = Some(time_signals.total_seconds.get_untracked()));
                tournament
                    .time_increment
                    .update_value(|v| *v = Some(time_signals.sec_per_move.get_untracked()));
            }
            TimeMode::Correspondence => {
                match time_signals.corr_mode.get_untracked() {
                    CorrespondenceMode::DaysPerMove => {
                        tournament.time_increment.update_value(|v| {
                            *v = Some(time_signals.corr_days.get_untracked() * 86400)
                        });
                        tournament.time_base.update_value(|v| *v = None);
                    }
                    CorrespondenceMode::TotalTimeEach => {
                        tournament.time_increment.update_value(|v| *v = None);
                        tournament.time_base.update_value(|v| {
                            *v = Some(time_signals.corr_days.get_untracked() * 86400)
                        });
                    }
                };
            }
        };
        if min_rating.get_untracked() < 500 {
            tournament.band_lower.update_untracked(|v| *v = None)
        } else {
            tournament
                .band_lower
                .update_untracked(|v| *v = Some(min_rating.get_untracked()))
        };
        if max_rating.get_untracked() > 2500 {
            tournament.band_upper.update_untracked(|v| *v = None)
        } else {
            tournament
                .band_upper
                .update_untracked(|v| *v = Some(max_rating.get_untracked()))
        };
        let details = TournamentDetails {
            name: tournament.name.get_untracked(),
            description: tournament.description.get_untracked(),
            scoring: tournament.scoring.get_untracked(),
            tiebreaker: tournament.tiebreaker.get_untracked(),
            invitees: vec![],
            seats: tournament.seats.get_untracked(),
            rounds: tournament.rounds.get_untracked(),
            joinable: tournament.joinable.get_untracked(),
            invite_only: tournament.invite_only.get_untracked(),
            mode: tournament.mode.get_untracked(),
            time_mode: tournament.time_mode.get_untracked(),
            time_base: tournament.time_base.get_value(),
            time_increment: tournament.time_increment.get_value(),
            band_upper: tournament.band_upper.get_untracked(),
            band_lower: tournament.band_lower.get_untracked(),
            series: tournament.series.get_untracked(),
            start_at: if organizer_start.get_untracked() {
                None
            } else {
                Some(tournament.start_at.get_untracked())
            },
        };
        if account.is_some() {
            let api = ApiRequests::new();
            let action = TournamentAction::Create(Box::new(details));
            api.tournament(action);
            let navigate = use_navigate();
            navigate("/tournaments", Default::default());
        }
    };
    let on_change: Callback<Event, ()> =
        Callback::from(update_from_input_parsed(time_signals.time_control));

    //let unused = move || {
    //    view! {
    //    <div class="flex flex-col">
    //        <div class="flex">
    //            <input
    //                on:change=move |_| organizer_start.update(|b| *b = !*b)
    //                type="checkbox"
    //                class="w-4 h-4 text-blue-600 bg-gray-100 rounded border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
    //                prop:checked=organizer_start
    //            />
    //            <label class="ml-2 text-sm font-medium text-gray-900 dark:text-gray-300">
    //                Manual start
    //            </label>
    //        </div>
    //        <Show when=move || !organizer_start()>
    //            <label for="start-time">Choose a start time:</label>
    //            <input
    //                type="datetime-local"
    //                id="start-time"
    //                name="start-time"
    //                attr:min=move || Local::now().format("%Y-%m-%dT%H:%M").to_string()
    //                attr:max=move || {
    //                    (Local::now() + Duration::weeks(12))
    //                        .format("%Y-%m-%dT%H:%M")
    //                        .to_string()
    //                }
    //
    //    //                value=(Local::now() + Duration::days(1))
    //    //                    .format("%Y-%m-%dT%H:%M")
    //    //                    .to_string()
    //    //                on:input=move |evt| {
    //    //                    if let Ok(date) = NaiveDateTime::parse_from_str(
    //    //                        &event_target_value(&evt),
    //    //                        "%Y-%m-%dT%H:%M",
    //    //                    ) {
    //    //                        tournament
    //    //                            .start_at
    //    //                            .update(|v| {
    //    //                                *v = DateTime::<Utc>::from_naive_utc_and_offset(date, Utc);
    //    //                            })
    //    //                    } else {
    //                        organizer_start.set(true)
    //                    }
    //                }
    //            />
    //
    //        </Show>
    //        <div class="p-1">
    //            Number of rounds:
    //            <InputSlider
    //                signal_to_update=tournament.rounds
    //                name="Rounds"
    //                min=1
    //                max=12
    //                step=1
    //            /> {tournament.rounds}
    //        </div>
    //    </div>}
    //};
    view! {
        <div class="flex justify-center items-center pt-10">
            <div class="container flex flex-col justify-between p-2 md:flex-row md:flex-wrap">
                <div class="basis-1/2">
                    <p class="text-3xl font-extrabold">Tournament settings:</p>
                    <div class="flex flex-col">
                        Tournament name:
                        <input
                            class="px-3 py-2 w-10/12 leading-tight rounded border shadow appearance-none focus:outline-none"
                            name="Tournament name"
                            type="text"
                            prop:value=tournament.name
                            placeholder="Tournament Name"
                            on:input=update_from_input(tournament.name)
                            attr:maxlength="128"
                        />
                    </div>

                    <div class="flex flex-col">
                        Description:
                        <textarea
                            class="px-3 py-2 w-10/12 leading-tight rounded border shadow appearance-none focus:outline-none"
                            name="Tournament description"
                            type="text"
                            prop:value=tournament.description
                            placeholder="Tournament description"
                            on:input=update_from_input(tournament.description)
                            attr:maxlength="2000"
                        ></textarea>
                    </div>

                    <div class="p-1">
                        Max number of players:
                        <InputSlider
                            signal_to_update=tournament.seats
                            name="Seats"
                            min=4
                            max=16
                            step=1
                        /> {tournament.seats}
                    </div>

                    <div>
                        Mode:
                        <select
                            class="bg-odd-light dark:bg-gray-700"
                            name="Tournament Mode"
                            on:change=update_from_input(tournament.mode)
                        >
                            <SelectOption value=tournament.mode is="Double Round Robin"/>

                        </select>
                    </div>
                    <div class="flex">
                        <input
                            on:change=move |_| tournament.invite_only.update(|b| *b = !*b)
                            type="checkbox"
                            class="w-4 h-4 text-blue-600 bg-gray-100 rounded border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600"
                            prop:checked=tournament.invite_only
                        />
                        <label class="ml-2 text-sm font-medium text-gray-900 dark:text-gray-300">
                            Invite Only
                        </label>
                    </div>

                </div>
                <div class="md:pl-2 basis-1/2">
                    <TimeSelect title=" Match settings:" time_signals on_change>
                        <SelectOption value=time_signals.time_control is="Real Time"/>
                        <SelectOption value=time_signals.time_control is="Correspondence"/>
                    </TimeSelect>
                    <div class="flex">{rating_string}</div>
                    <div class="flex">
                        <div class="flex gap-1 mx-1">
                            <label class="flex items-center">
                                <InputSlider
                                    signal_to_update=min_rating
                                    name="Min rating"
                                    min=400
                                    max=Signal::derive(move || { max_rating() - 100 })
                                    step=100
                                />
                            </label>
                            <label class="flex items-center">
                                <InputSlider
                                    signal_to_update=max_rating
                                    name="Max rating"
                                    min=Signal::derive(move || { min_rating() + 100 })
                                    max=2600
                                    step=100
                                />
                            </label>
                        </div>
                    </div>
                </div>
                <div>
                    <button class=BUTTON_STYLE on:click=create>
                        "Create Tournament"
                    </button>
                </div>
            </div>
        </div>
    }
}
