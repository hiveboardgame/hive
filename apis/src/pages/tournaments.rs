use crate::{
    common::TournamentAction,
    common::TournamentResponseDepth::Abstract,
    components::molecules::tournament_row::TournamentRow,
    providers::{tournaments::TournamentStateContext, websocket::WebsocketContext, ApiRequests},
};
use leptos::*;
use leptos_use::core::ConnectionReadyState;

#[component]
pub fn Tournaments() -> impl IntoView {
    let tournament = expect_context::<TournamentStateContext>();
    let ws = expect_context::<WebsocketContext>();
    create_effect(move |_| {
        if ws.ready_state.get() == ConnectionReadyState::Open {
            let api = ApiRequests::new();
            api.tournament(TournamentAction::GetAll(Abstract));
        };
    });
    let search = RwSignal::new("".to_string());
    view! {
        <div class="pt-10">
            <div class="container px-4 mx-auto">
                <input
                    type="text"
                    class="items-center p-2 mx-2 my-2 w-5/6"
                    placeholder="Search by tournament name or description"
                    on:input=move |ev| search.set(event_target_value(&ev))
                    value=search
                />
                <For
                    each=move || { tournament.summary.get().tournaments }
                    key=move |(nanoid, tournament)| {
                        (nanoid.to_owned(), tournament.updated_at, search())
                    }
                    let:tournament
                    children=move |tournament| {
                        if search().is_empty()
                            || tournament.1.name.to_lowercase().contains(&search().to_lowercase())
                        {
                            view! { <TournamentRow tournament=tournament.1/> }
                        } else {
                            "".into_view()
                        }
                    }
                />

            </div>
        </div>
    }
}
