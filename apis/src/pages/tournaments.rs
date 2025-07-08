use crate::functions::tournaments::get_all_abstract;
use crate::pages::tournament::BUTTON_STYLE;
use crate::{components::molecules::tournament_row::TournamentRow, providers::AuthContext};
use leptos::either::Either;
use leptos::prelude::*;
use shared_types::{TournamentSortOrder, TournamentStatus};

#[derive(Clone, PartialEq, Eq, Hash)]
enum TournamentFilter {
    All,
    Status(TournamentStatus),
    MyTournaments,
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
    let auth_context = expect_context::<AuthContext>();
    let logged_in = move || auth_context.user.get().is_some();
    let filter = RwSignal::new(TournamentFilter::Status(TournamentStatus::NotStarted));
    let search = RwSignal::new("".to_string());
    let tournament_resource =
        OnceResource::new(get_all_abstract(TournamentSortOrder::CreatedAtDesc));
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
                <div class="flex flex-wrap justify-center content-center -mx-2 mb-4 space-x-4 w-full">
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
                    <Show when=logged_in>
                        <button
                            class=move || get_button_classes(
                                TournamentFilter::MyTournaments,
                                filter.get(),
                            )
                            on:click=move |_| { filter.set(TournamentFilter::MyTournaments) }
                        >
                            {"My\u{00A0}Tournaments"}
                        </button>
                    </Show>
                </div>
                <Transition fallback=move || {
                    view! { <div class="flex justify-center">"Loading tournaments..."</div> }
                }>
                    {move || {
                        tournament_resource
                            .get()
                            .map(|tournaments| {
                                if let Ok(tournaments) = tournaments {
                                    Either::Left(
                                        view! {
                                            <For
                                                each=move || {
                                                    let mut v: Vec<_> = tournaments
                                                        .clone()
                                                        .into_iter()
                                                        .filter(|t| {
                                                            match filter.get() {
                                                                TournamentFilter::All => true,
                                                                TournamentFilter::Status(status) => t.status == status,
                                                                TournamentFilter::MyTournaments => auth_context.user.get_untracked().is_some_and(|u|t.player_list.contains(&u.id)),
                                                            }
                                                        })
                                                        .collect();
                                                    v.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                                                    v
                                                }
                                                key=move |tournament| {
                                                    (tournament.updated_at, search(), filter.get())
                                                }
                                                children=move |tournament| {
                                                    if search().is_empty()
                                                        || tournament
                                                            .name
                                                            .to_lowercase()
                                                            .contains(&search().to_lowercase())
                                                    {
                                                        Either::Left(
                                                            view! { <TournamentRow tournament=tournament.clone() /> },
                                                        )
                                                    } else {
                                                        Either::Right("")
                                                    }
                                                }
                                            />
                                        },
                                    )
                                } else {
                                    Either::Right(
                                        view! {
                                            <div class="flex justify-center">
                                                {"Error loading tournaments"}
                                            </div>
                                        },
                                    )
                                }
                            })
                    }}
                </Transition>
            </div>
        </div>
    }
}
