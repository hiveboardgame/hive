use crate::pages::tournament::BUTTON_STYLE;
use crate::providers::ApiRequestsProvider;
use crate::{
    common::TournamentAction,
    common::TournamentResponseDepth::Abstract,
    components::molecules::tournament_row::TournamentRow,
    providers::{tournaments::TournamentStateContext, websocket::WebsocketContext},
};
use leptos::either::Either;
use leptos::prelude::*;
use leptos_use::core::ConnectionReadyState;
use shared_types::{TournamentSortOrder, TournamentStatus};

#[derive(Clone, PartialEq, Eq, Hash)]
enum TournamentFilter {
    All,
    Status(TournamentStatus),
}

fn get_button_classes(current: TournamentFilter, selected: TournamentFilter) -> &'static str {
    if current == selected {
        return "px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600";
    }

    // default state
    BUTTON_STYLE
}

#[component]
pub fn Tournaments() -> impl IntoView {
    let tournament = expect_context::<TournamentStateContext>();
    let ws = expect_context::<WebsocketContext>();
    let filter = RwSignal::new(TournamentFilter::Status(TournamentStatus::NotStarted));
    let api = expect_context::<ApiRequestsProvider>().0;
    Effect::new(move |_| {
        if ws.ready_state.get() == ConnectionReadyState::Open {
            let api = api.get();
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
                <div class="flex justify-center content-center -mx-2 mb-4 space-x-4 w-full">
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
                                    TournamentFilter::All => true,
                                    TournamentFilter::Status(status) => t.status == status,
                                }
                            })
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
                            Either::Left(view! { <TournamentRow tournament=tournament.clone() /> })
                        } else {
                            Either::Right("")
                        }
                    }
                />
            </div>
        </div>
    }
}
