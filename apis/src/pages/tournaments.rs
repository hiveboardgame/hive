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
                    each=move || {
                        let mut v: Vec<_> = tournament
                            .summary
                            .get()
                            .tournaments
                            .into_iter()
                            .collect();
                        v.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
                        v
                    }

                    key=move |(nanoid, tournament)| {
                        (nanoid.to_owned(), tournament.updated_at, search())
                    }

                    children=move |(_id, tournament)| {
                        if search().is_empty()
                            || tournament.name.to_lowercase().contains(&search().to_lowercase())
                        {
                            view! { <TournamentRow tournament=tournament.clone()/> }
                        } else {
                            "".into_view()
                        }
                    }
                />

            </div>
        </div>
    }
}
