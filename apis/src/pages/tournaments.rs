use crate::{
    common::TournamentAction,
    providers::{tournaments::TournamentStateSignal, ApiRequests, AuthContext},
};
use leptos::*;
use shared_types::{TimeMode, TournamentDetails};

const BUTTON_STYLE: &str = "flex w-full gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95";

#[component]
pub fn Tournaments() -> impl IntoView {
    let tournament = expect_context::<TournamentStateSignal>();
    view! {
        <div class="pt-10">Tournaments</div>
        <button
            class=BUTTON_STYLE
            on:click=move |_| {
                let auth_context = expect_context::<AuthContext>();
                let account = match (auth_context.user)() {
                    Some(Ok(Some(account))) => Some(account),
                    _ => None,
                };
                let details = TournamentDetails {
                    name: String::from("TestTournament"),
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
                key=|(nanoid, tournament)| nanoid.to_owned()
                let:tournament
            >
                <div>{tournament.1.nanoid}</div>
            </For>
        </div>
    }
}
