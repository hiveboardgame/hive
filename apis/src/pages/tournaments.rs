use crate::{
    common::TournamentAction,
    providers::{
        tournaments::TournamentStateSignal, websocket::WebsocketContext, ApiRequests, AuthContext,
    },
};
use leptos::*;
use leptos_use::core::ConnectionReadyState;
use shared_types::{TimeMode, TournamentDetails};
use uuid::Uuid;

const BUTTON_STYLE: &str = "flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95";

#[derive(Debug, Clone, Copy)]
pub struct TournamentParams {
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

#[component]
pub fn Tournaments() -> impl IntoView {
    let tournament = expect_context::<TournamentStateSignal>();
    let ws = expect_context::<WebsocketContext>();
    create_effect(move |_| {
        if ws.ready_state.get() == ConnectionReadyState::Open {
            let api = ApiRequests::new();
            api.tournament(TournamentAction::GetAll);
        };
    });
    view! {
        <div class="pt-10 m-2">Tournaments</div>
        <button
            class=BUTTON_STYLE
            on:click=move |_| {
                let auth_context = expect_context::<AuthContext>();
                let account = match (auth_context.user)() {
                    Some(Ok(Some(account))) => Some(account),
                    _ => None,
                };
                let details = TournamentDetails {
                    name: String::from("Test"),
                    description: String::from("Great tournament"),
                    scoring: String::from("meh"),
                    tiebreaker: Vec::new(),
                    invitees: Vec::new(),
                    seats: 4,
                    rounds: 2,
                    joinable: true,
                    invite_only: false,
                    mode: String::from("RoundRobin"),
                    time_mode: TimeMode::RealTime,
                    time_base: Some(60),
                    time_increment: Some(2),
                    band_upper: None,
                    band_lower: None,
                    start_at: None,
                    series: None,
                };
                if account.is_some() {
                    let api = ApiRequests::new();
                    let action = TournamentAction::Create(Box::new(details));
                    api.tournament(action);
                }
            }
        >

            "Create Tournament"
        </button>
        <div>
            <For
                each=move || { tournament.signal.get().tournaments }
                key=|(nanoid, _tournament)| nanoid.to_owned()
                let:tournament
            >
                <div class="flex relative justify-between">
                    <a
                        class="text-blue-500 hover:underline"
                        href=format!("/tournament/{}", tournament.1.nanoid)
                    >
                        {tournament.1.nanoid.clone()}
                    </a>
                    <button
                        class=BUTTON_STYLE
                        on:click=move |_| {
                            let auth_context = expect_context::<AuthContext>();
                            let account = match (auth_context.user)() {
                                Some(Ok(Some(account))) => Some(account),
                                _ => None,
                            };
                            if account.is_some() {
                                let api = ApiRequests::new();
                                let action = TournamentAction::Join(tournament.1.nanoid.clone());
                                api.tournament(action);
                            }
                        }
                    >

                        "Join Tournament"
                    </button>
                </div>
            </For>
        </div>
    }
}
