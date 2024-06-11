use crate::common::TournamentAction;
use crate::providers::ApiRequests;
use crate::providers::AuthContext;
use chrono::Utc;
use leptos::*;
use shared_types::TimeMode;
use shared_types::TournamentDetails;
use uuid::Uuid;

const BUTTON_STYLE: &str = "flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95";

#[derive(Debug, Clone, Copy)]
pub struct TournamentParamSignals {
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
}

impl TournamentParamSignals {
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
            mode: RwSignal::new(String::from("RR")),
            time_mode: RwSignal::new(TimeMode::RealTime),
            time_base: store_value(Some(60)),
            time_increment: store_value(Some(0)),
            band_upper: RwSignal::new(None),
            band_lower: RwSignal::new(None),
            series: RwSignal::new(None),
        }
    }
}

impl Default for TournamentParamSignals {
    fn default() -> Self {
        Self::new()
    }
}

#[component]
pub fn TournamentCreate() -> impl IntoView {
    let tournament = TournamentParamSignals::default();
    let name = move |evt| tournament.name.update(|v| *v = event_target_value(&evt));
    let description = move |evt| {
        tournament
            .description
            .update(|v| *v = event_target_value(&evt))
    };
    let create = move |_| {
        let auth_context = expect_context::<AuthContext>();
        let account = match (auth_context.user)() {
            Some(Ok(Some(account))) => Some(account),
            _ => None,
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
            start_at: Some(Utc::now()),
        };
        if account.is_some() {
            let api = ApiRequests::new();
            let action = TournamentAction::Create(Box::new(details));
            api.tournament(action);
        }
    };
    view! {
        <div class="flex flex-col pt-10">
            <div>
                Tournament name:
                <input
                    class="px-3 py-2 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                    name="Tournament name"
                    type="text"
                    prop:value=tournament.name
                    placeholder="Tournament Name"
                    on:input=name
                />
            </div>

            <div>
                Description:
                <input
                    class="px-3 py-2 w-full leading-tight rounded border shadow appearance-none focus:outline-none"
                    name="Tournament description"
                    type="text"
                    prop:value=tournament.description
                    placeholder="Tournament description"
                    on:input=description
                />
            </div>

            <div>Max number of players:</div>

            <div>Mode: round robin</div>
        </div>
        <div>
            <button class=BUTTON_STYLE on:click=create>
                "Create Tournament"
            </button>
        </div>
    }
}
