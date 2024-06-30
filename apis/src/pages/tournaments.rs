use crate::{
    common::TournamentAction,
    components::molecules::tournament_row::TournamentRow,
    providers::{tournaments::TournamentStateSignal, websocket::WebsocketContext, ApiRequests},
};
use leptos::*;
use leptos_use::core::ConnectionReadyState;

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
        <div class="pt-10">
            <div class="container px-4 mx-auto">
                Tournaments
                <For
                    each=move || { tournament.signal.get().tournaments }
                    key=|(nanoid, tournament)| { (nanoid.to_owned(), tournament.updated_at) }
                    let:tournament
                >
                    <TournamentRow tournament=tournament.1/>
                </For>
            </div>
        </div>
    }
}
