use std::thread::current;

use crate::pages::tournament::BUTTON_STYLE;
use crate::{
    common::TournamentAction,
    common::TournamentResponseDepth::Abstract,
    components::molecules::tournament_row::TournamentRow,
    providers::{tournaments::TournamentStateContext, websocket::WebsocketContext, ApiRequests},
};
use leptos::*;
use leptos_use::core::ConnectionReadyState;
use shared_types::{TournamentSortOrder, TournamentStatus};
use crate::providers::navigation_controller::NavigationControllerSignal;
use leptos::logging::log;
use crate::providers::AuthContext;


#[derive(Clone, PartialEq, Eq, Hash)]
enum TournamentFilter {
    All,
    Status(TournamentStatus),
    MyTournaments,
}fn get_button_classes(current: TournamentFilter, selected: TournamentFilter) -> &'static str {
    if current == selected {
        return "px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
    }

    // default state
    return BUTTON_STYLE
}

#[component]
pub fn Tournaments() -> impl IntoView {
    let tournament = expect_context::<TournamentStateContext>();
    let ws = expect_context::<WebsocketContext>();
    let navi = expect_context::<NavigationControllerSignal>();
    let auth_context = expect_context::<AuthContext>();
    let username = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user.username),
        _ => None
    };
    let filter = RwSignal::new(TournamentFilter::Status(TournamentStatus::NotStarted));
    create_effect(move |_| {
        if ws.ready_state.get() == ConnectionReadyState::Open {
            let api = ApiRequests::new();
            api.tournament(TournamentAction::GetAll(
                Abstract,
                TournamentSortOrder::CreatedAtDesc,
            ));
        };
    });
    let search = RwSignal::new("".to_string());
    view! {
        <div class="pt-10">
            <div class="container px-4 mx-auto">
                <input
                    type="text"
                    class="items-center p-2 mx-2 my-2 w-5/6"
                    placeholder="Search tournaments by name"
                    on:input=move |ev| search.set(event_target_value(&ev))
                    value=search
                />                
                <div class="flex justify-center space-x-4 mb-4 w-full -mx-2 content-center">
                    <button
                        class=move || get_button_classes(TournamentFilter::All, filter.get())
                        on:click=move |_| filter.set(TournamentFilter::All)
                    >
                        "All"
                    </button>
                    <button
                        class=move || get_button_classes(
                            TournamentFilter::Status(TournamentStatus::NotStarted),
                            filter.get(),
                        )
                        on:click=move |_| {
                            filter.set(TournamentFilter::Status(TournamentStatus::NotStarted))
                        }
                    >
                        "Future"
                    </button>
                    <button
                        class=move || get_button_classes(
                            TournamentFilter::Status(TournamentStatus::InProgress),
                            filter.get(),
                        )
                        on:click=move |_| {
                            filter.set(TournamentFilter::Status(TournamentStatus::InProgress))
                        }
                    >
                        {"In\u{00A0}Progress"}
                    </button>
                    <button
                        class=move || get_button_classes(
                            TournamentFilter::Status(TournamentStatus::Finished),
                            filter.get(),
                        )
                        on:click=move |_| {
                            filter.set(TournamentFilter::Status(TournamentStatus::Finished))
                        }
                    >
                        "Completed"
                    </button>
                    <button
    class=move || get_button_classes(
        TournamentFilter::MyTournaments,
        filter.get(),
    )
    on:click=move |_| filter.set(TournamentFilter::MyTournaments)
>
    "My Tournaments"
</button>

                </div>
                <For
                    each=move || {
                        let mut v: Vec<_> = tournament
                            .summary
                            .get()
                            .tournaments
                            .into_iter()
                            .filter(|(_, t)| {
                                match filter.get() {
                                    TournamentFilter::All => {
                                        log!("test me 123");
                                        log!("players #: {}", t.player_list.len());
                                        log!("players: {}", t.player_list.join(", "));
                                        log!("current user: {}", username().unwrap_or_default());
                                        true
                                    },
                                    TournamentFilter::Status(status) => t.status == status,
                                    TournamentFilter::MyTournaments => {
                                        log!("test me 123");
                                        t.player_list.contains(&username().unwrap_or_default().clone())
                                    }
                                }                            })
                            .collect();
                        v.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at));
                        v
                    }
                    key=move |(nanoid, tournament)| {
                        (nanoid.to_owned(), tournament.updated_at, search(), filter.get())
                    }
                    children=move |(_id, tournament)| {
                        if search().is_empty()
                            || tournament.name.to_lowercase().contains(&search().to_lowercase())
                        {
                            view! { <TournamentRow tournament=tournament.clone() /> }
                        } else {
                            "".into_view()
                        }
                    }
                />
            </div>
        </div>
    }
}
