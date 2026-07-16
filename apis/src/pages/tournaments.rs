use crate::{
    components::{
        layouts::{
            page_header::PageHeader,
            page_shell::{PageShell, PageShellVariant},
        },
        molecules::{empty_state::EmptyState, panel::Panel, tournament_row::TournamentRow},
    },
    functions::tournaments::{get_by_status, get_hosting_tournaments, get_joined_tournaments},
    providers::{AuthContext, AuthIdentity},
    responses::TournamentAbstractResponse,
};
use leptos::{either::Either, prelude::*};
use leptos_router::hooks::use_location;
use shared_types::{TournamentSortOrder, TournamentStatus};

fn tournament_tab_class(current_path: &str, target_path: &str) -> &'static str {
    let is_active = current_path == target_path
        || (current_path == "/tournaments" && target_path == "/tournaments/future");

    if is_active {
        "ui-button ui-button-primary ui-button-md no-link-style"
    } else {
        "ui-button ui-button-secondary ui-button-md no-link-style"
    }
}

#[component]
pub fn Tournaments(children: Children) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let location = use_location();
    let logged_in = move || matches!(auth_context.identity.get(), Some(AuthIdentity::User(_)));

    view! {
        <PageShell variant=PageShellVariant::Dashboard>
            <div class="flex flex-col gap-4 w-full max-w-5xl">
                <PageHeader
                    title="Tournaments"
                    subtitle="Browse upcoming, active, and completed tournaments."
                />
                <div class="flex flex-wrap gap-2">
                    <a
                        href="/tournaments/future"
                        class=move || tournament_tab_class(
                            &location.pathname.get(),
                            "/tournaments/future",
                        )
                    >
                        "Future"
                    </a>
                    <a
                        href="/tournaments/inprogress"
                        class=move || tournament_tab_class(
                            &location.pathname.get(),
                            "/tournaments/inprogress",
                        )
                    >
                        {"In\u{00A0}Progress"}
                    </a>
                    <a
                        href="/tournaments/finished"
                        class=move || tournament_tab_class(
                            &location.pathname.get(),
                            "/tournaments/finished",
                        )
                    >
                        "Completed"
                    </a>
                    <Show when=logged_in>
                        <a
                            href="/tournaments/joined"
                            class=move || tournament_tab_class(
                                &location.pathname.get(),
                                "/tournaments/joined",
                            )
                        >
                            "Joined"
                        </a>
                        <a
                            href="/tournaments/hosting"
                            class=move || tournament_tab_class(
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
        </PageShell>
    }
}

#[component]
pub fn TournamentList(
    tournament_resource: OnceResource<Result<Vec<TournamentAbstractResponse>, ServerFnError>>,
) -> impl IntoView {
    let search = RwSignal::new(String::new());

    view! {
        <Panel body_class="space-y-4" class="w-full">
            <input
                type="text"
                class="ui-field-input"
                placeholder="Search tournaments by name"
                on:input=move |ev| search.set(event_target_value(&ev))
                prop:value=search
            />
            <Transition fallback=move || {
                view! { <EmptyState title="Loading tournaments..." /> }
            }>
                {move || {
                    tournament_resource
                        .get()
                        .map(|tournaments| {
                            match tournaments {
                                Ok(tournaments) => {
                                    let search_term = search.get().to_lowercase();
                                    Either::Left(

                                        view! {
                                            <For
                                                each=move || {
                                                    tournaments
                                                        .iter()
                                                        .filter(|t| {
                                                            search_term.is_empty()
                                                                || t.name.to_lowercase().contains(&search_term)
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
                                        view! { <EmptyState title="Error loading tournaments" /> },
                                    )
                                }
                            }
                        })
                }}
            </Transition>
        </Panel>
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
