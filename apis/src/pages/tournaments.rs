use crate::components::molecules::tournament_row::TournamentRow;
use crate::functions::tournaments::{
    get_by_status, get_hosting_tournaments, get_joined_tournaments,
};
use crate::providers::AuthContext;
use crate::responses::TournamentAbstractResponse;
use leptos::either::Either;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use shared_types::{TournamentSortOrder, TournamentStatus};

fn get_button_classes(current_path: &str, target_path: &str) -> String {
    let base_classes = "no-link-style px-4 py-2 rounded";
    let is_active = current_path == target_path
        || (current_path == "/tournaments" && target_path == "/tournaments/future");

    if is_active {
        format!("{base_classes} bg-blue-500 text-white hover:bg-blue-600")
    } else {
        format!("{base_classes} font-bold text-white hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 bg-button-dawn dark:bg-button-twilight")
    }
}

#[component]
pub fn Tournaments(children: Children) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let location = use_location();
    let logged_in = move || auth_context.user.get().is_some();

    view! {
        <div class="pt-10">
            <div class="container flex flex-col items-center px-4 mx-auto">
                <div class="flex flex-wrap gap-1 justify-center content-center m-4 w-full">
                    <a
                        href="/tournaments/future"
                        class=move || get_button_classes(
                            &location.pathname.get(),
                            "/tournaments/future",
                        )
                    >
                        "Future"
                    </a>
                    <a
                        href="/tournaments/inprogress"
                        class=move || get_button_classes(
                            &location.pathname.get(),
                            "/tournaments/inprogress",
                        )
                    >
                        {"In\u{00A0}Progress"}
                    </a>
                    <a
                        href="/tournaments/finished"
                        class=move || get_button_classes(
                            &location.pathname.get(),
                            "/tournaments/finished",
                        )
                    >
                        "Completed"
                    </a>
                    <Show when=logged_in>
                        <a
                            href="/tournaments/joined"
                            class=move || get_button_classes(
                                &location.pathname.get(),
                                "/tournaments/joined",
                            )
                        >
                            "Joined"
                        </a>
                        <a
                            href="/tournaments/hosting"
                            class=move || get_button_classes(
                                &location.pathname.get(),
                                "/tournaments/hosting",
                            )
                        >
                            "Hosting"
                        </a>
                    </Show>
                </div>
                {children()}
            </div>
        </div>
    }
}

#[component]
pub fn TournamentList(
    tournament_resource: OnceResource<Result<Vec<TournamentAbstractResponse>, ServerFnError>>,
) -> impl IntoView {
    let search = RwSignal::new(String::new());

    view! {
        <div class="flex flex-col items-center w-full">
            <input
                type="text"
                class="items-center p-2 mx-2 my-2 w-5/6"
                placeholder="Search tournaments by name"
                on:input=move |ev| search.set(event_target_value(&ev))
                prop:value=search
            />
            <Transition fallback=move || {
                view! { <div class="flex justify-center">"Loading tournaments..."</div> }
            }>
                {move || {
                    tournament_resource
                        .get()
                        .map(|tournaments| {
                            match tournaments {
                                Ok(tournaments) => {
                                    let search_term = search.get();
                                    Either::Left(

                                        view! {
                                            <For
                                                each=move || {
                                                    tournaments
                                                        .iter()
                                                        .filter(|t| {
                                                            search_term.is_empty()
                                                                || t
                                                                    .name
                                                                    .to_lowercase()
                                                                    .contains(&search_term.to_lowercase())
                                                        })
                                                        .cloned()
                                                        .collect::<Vec<_>>()
                                                }
                                                key=|tournament| tournament.tournament_id.clone()
                                                children=move |tournament| {
                                                    view! { <TournamentRow tournament /> }
                                                }
                                            />
                                        },
                                    )
                                }
                                Err(_) => {
                                    Either::Right(
                                        view! {
                                            <div class="flex justify-center">
                                                "Error loading tournaments"
                                            </div>
                                        },
                                    )
                                }
                            }
                        })
                }}
            </Transition>
        </div>
    }
}

#[component]
pub fn TournamentsByStatus(status: TournamentStatus) -> impl IntoView {
    let tournament_resource =
        OnceResource::new(get_by_status(status, TournamentSortOrder::CreatedAtDesc));

    view! { <TournamentList tournament_resource /> }
}

#[component]
pub fn JoinedTournaments() -> impl IntoView {
    let tournament_resource =
        OnceResource::new(get_joined_tournaments(TournamentSortOrder::CreatedAtDesc));

    view! { <TournamentList tournament_resource /> }
}

#[component]
pub fn HostingTournaments() -> impl IntoView {
    let tournament_resource =
        OnceResource::new(get_hosting_tournaments(TournamentSortOrder::CreatedAtDesc));

    view! { <TournamentList tournament_resource /> }
}
