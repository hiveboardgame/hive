use crate::{
    common::TournamentAction,
    providers::{
        tournaments::TournamentStateSignal, websocket::WebsocketContext, ApiRequests, AuthContext,
    },
};
use leptos::*;
use leptos_use::core::ConnectionReadyState;
use shared_types::{TimeMode, TournamentDetails};

const BUTTON_STYLE: &str = "flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95";

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
                </div>
            </For>
        </div>
    }
}
