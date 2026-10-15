use crate::{
    common::{ServerMessage, ServerResult, TournamentUpdate},
    providers::websocket::WebsocketContext,
};
mod snapshot;

use self::snapshot::TournamentBrowseSnapshot;
use crate::{
    components::{
        layouts::{
            page_header::PageHeader,
            page_shell::{PageShell, PageShellVariant},
        },
        molecules::{empty_state::EmptyState, panel::Panel},
    },
    functions::tournaments::browse_tournaments,
    i18n::*,
    providers::{AuthContext, AuthIdentity},
    responses::{TournamentBrowsePage, TournamentCategory, TournamentFamily, TournamentTimeFilter},
};
use leptos::{either::Either, prelude::*};
use leptos_router::{
    components::{Redirect, A},
    hooks::{use_location, use_navigate},
    location::{State, Url},
    NavigateOptions,
};
use leptos_use::{use_timeout_fn, UseTimeoutFnReturn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

fn tournament_tab_class(active: bool) -> &'static str {
    if active {
        "ui-button ui-button-primary ui-button-md no-link-style"
    } else {
        "ui-button ui-button-secondary ui-button-md no-link-style"
    }
}

fn mine_tab_class(active: bool) -> &'static str {
    if active {
        "shrink-0 border-b-2 border-pillbug-teal px-3 py-2 text-sm font-semibold no-link-style"
    } else {
        "shrink-0 border-b-2 border-transparent px-3 py-2 text-sm text-gray-600 dark:text-gray-300 no-link-style"
    }
}

fn browse_url(
    path: &str,
    search: &str,
    page: usize,
    format: Option<TournamentFamily>,
    mode: TournamentTimeFilter,
) -> String {
    let search = search.trim();
    let mode = normalized_mode(format, mode);
    let mut query = Vec::new();
    if !search.is_empty() {
        query.push(format!("q={}", Url::escape(search)));
    }
    if let Some(format) = format {
        query.push(format!("format={}", format.as_str()));
    }
    if mode != TournamentTimeFilter::Both {
        query.push(format!("mode={}", mode.as_str()));
    }
    if page > 1 {
        query.push(format!("page={page}"));
    }
    if query.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", query.join("&"))
    }
}

fn normalized_mode(
    format: Option<TournamentFamily>,
    mode: TournamentTimeFilter,
) -> TournamentTimeFilter {
    if format == Some(TournamentFamily::Arena) {
        TournamentTimeFilter::Live
    } else {
        mode
    }
}

fn identity_key(identity: Option<AuthIdentity>) -> (bool, Option<Uuid>) {
    (identity.is_some(), identity.and_then(AuthIdentity::user_id))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct BrowseRequest {
    path: String,
    search: String,
    page: usize,
    format: Option<TournamentFamily>,
    mode: TournamentTimeFilter,
    identity: (bool, Option<Uuid>),
    revision: u64,
}

fn category_path(category: TournamentCategory) -> &'static str {
    match category {
        TournamentCategory::Upcoming => "/tournaments",
        TournamentCategory::InProgress => "/tournaments/in-progress",
        TournamentCategory::Finished => "/tournaments/finished",
        TournamentCategory::Joined => "/tournaments/mine/joined",
        TournamentCategory::Organizing => "/tournaments/mine/organizing",
        TournamentCategory::Invitations => "/tournaments/mine/invitations",
        TournamentCategory::History => "/tournaments/mine/history",
    }
}

fn parsed_page(raw: Option<String>) -> usize {
    raw.and_then(|page| page.parse::<usize>().ok())
        .filter(|page| *page > 0)
        .unwrap_or(1)
}

fn replace_navigation_options() -> NavigateOptions {
    NavigateOptions {
        resolve: true,
        replace: true,
        scroll: false,
        state: State::new(None),
    }
}

#[component]
pub fn Tournaments(children: Children) -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let location = use_location();
    let logged_in = move || matches!(auth_context.identity.get(), Some(AuthIdentity::User(_)));
    let path_is = move |target: &'static str| location.pathname.get() == target;
    let mine_is_active = move || location.pathname.get().starts_with("/tournaments/mine/");
    let tab_url = move |path| {
        let query = location.query.get();
        browse_url(
            path,
            &query.get("q").unwrap_or_default(),
            1,
            query.get("format").and_then(|value| value.parse().ok()),
            query
                .get("mode")
                .and_then(|value| value.parse().ok())
                .unwrap_or_default(),
        )
    };

    view! {
        <PageShell variant=PageShellVariant::Content>
            <div class="flex flex-col gap-3 w-full min-w-0 sm:gap-4">
                <div class="flex flex-wrap gap-3 justify-between items-start">
                    <PageHeader
                        title=move || t_string!(i18n, tournaments.list.title)
                        subtitle=move || t_string!(i18n, tournaments.list.subtitle)
                    />
                    // TODO: i18n once copy is approved.
                    <A
                        href="/tournaments/create"
                        attr:class="ui-button ui-button-primary ui-button-md no-link-style"
                    >
                        "Create tournament"
                    </A>
                </div>
                <nav class="flex flex-wrap gap-2">
                    // TODO: i18n once copy is approved.
                    <A
                        href=move || tab_url("/tournaments")
                        attr:class=move || tournament_tab_class(path_is("/tournaments"))
                    >
                        "Upcoming"
                    </A>
                    // TODO: i18n once copy is approved.
                    <A
                        href=move || tab_url("/tournaments/in-progress")
                        attr:class=move || tournament_tab_class(path_is("/tournaments/in-progress"))
                    >
                        "In progress"
                    </A>
                    // TODO: i18n once copy is approved.
                    <A
                        href=move || tab_url("/tournaments/finished")
                        attr:class=move || tournament_tab_class(path_is("/tournaments/finished"))
                    >
                        "Finished"
                    </A>
                    <Show when=logged_in>
                        // TODO: i18n once copy is approved.
                        <A
                            href=move || tab_url("/tournaments/mine/joined")
                            attr:class=move || tournament_tab_class(mine_is_active())
                        >
                            "Mine"
                        </A>
                    </Show>
                </nav>
                <Show when=mine_is_active>
                    <nav class="flex overflow-x-auto gap-0 min-w-0 border-b border-gray-200 dark:border-gray-700">
                        // TODO: i18n once copy is approved.
                        <A
                            href=move || tab_url("/tournaments/mine/joined")
                            attr:class=move || mine_tab_class(path_is("/tournaments/mine/joined"))
                        >
                            "Joined"
                        </A>
                        // TODO: i18n once copy is approved.
                        <A
                            href=move || tab_url("/tournaments/mine/organizing")
                            attr:class=move || mine_tab_class(
                                path_is("/tournaments/mine/organizing"),
                            )
                        >
                            "Organizing"
                        </A>
                        // TODO: i18n once copy is approved.
                        <A
                            href=move || tab_url("/tournaments/mine/invitations")
                            attr:class=move || mine_tab_class(
                                path_is("/tournaments/mine/invitations"),
                            )
                        >
                            "Invitations"
                        </A>
                        // TODO: i18n once copy is approved.
                        <A
                            href=move || tab_url("/tournaments/mine/history")
                            attr:class=move || mine_tab_class(path_is("/tournaments/mine/history"))
                        >
                            "History"
                        </A>
                    </nav>
                </Show>
                {children()}
            </div>
        </PageShell>
    }
}

#[component]
pub fn MineTournamentsRedirect() -> impl IntoView {
    view! { <LegacyTournamentsRedirect path="/tournaments/mine/joined" /> }
}

#[component]
pub fn LegacyTournamentsRedirect(path: &'static str) -> impl IntoView {
    let location = use_location();
    let query = location.search.get_untracked();
    let path = if query.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{query}")
    };
    view! {
        <Redirect
            path
            options=NavigateOptions {
                replace: true,
                ..Default::default()
            }
        />
    }
}

fn empty_list_title(
    i18n: leptos_i18n::I18nContext<Locale, I18nKeys>,
    category: TournamentCategory,
) -> String {
    match category {
        TournamentCategory::Upcoming => {
            t_string!(i18n, tournaments.list.empty_upcoming).to_string()
        }
        TournamentCategory::InProgress => {
            t_string!(i18n, tournaments.list.empty_in_progress).to_string()
        }
        TournamentCategory::Finished => {
            t_string!(i18n, tournaments.list.empty_finished).to_string()
        }
        // TODO: i18n once copy is approved.
        TournamentCategory::Joined => String::from("No joined tournaments"),
        // TODO: i18n once copy is approved.
        TournamentCategory::Organizing => String::from("No tournaments you organize"),
        // TODO: i18n once copy is approved.
        TournamentCategory::Invitations => String::from("No pending invitations"),
        // TODO: i18n once copy is approved.
        TournamentCategory::History => String::from("No tournament history"),
    }
}

#[component]
pub fn TournamentList(category: TournamentCategory) -> impl IntoView {
    let i18n = use_i18n();
    let location = use_location();
    let navigate = use_navigate();
    let auth = expect_context::<AuthContext>();
    let queries = location.query;
    let revision = RwSignal::new(0_u64);
    let current = Memo::new(move |_| {
        let query = queries.get();
        let format = query.get("format").and_then(|value| value.parse().ok());
        let mode = query
            .get("mode")
            .and_then(|value| value.parse().ok())
            .unwrap_or_default();
        BrowseRequest {
            path: location.pathname.get(),
            search: query.get("q").unwrap_or_default().trim().to_owned(),
            page: parsed_page(query.get("page")),
            format,
            mode: normalized_mode(format, mode),
            identity: identity_key(auth.identity.get()),
            revision: revision.get(),
        }
    });
    let search = Signal::derive(move || current.get().search);
    let format = Signal::derive(move || current.get().format);
    let mode = Signal::derive(move || current.get().mode);
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            let request = current.get();
            if request.path.trim_end_matches('/') != category_path(category) {
                return;
            }
            let canonical = browse_url(
                &request.path,
                &request.search,
                request.page,
                request.format,
                request.mode,
            );
            let raw_search = location.search.get();
            let actual = if raw_search.is_empty() {
                request.path
            } else {
                format!("{}?{raw_search}", request.path)
            };
            if canonical != actual {
                navigate(&canonical, replace_navigation_options());
            }
        });
    }
    let resource = Resource::new(
        move || current.get(),
        move |request| async move {
            let response = if request.path.trim_end_matches('/') == category_path(category) {
                Some(
                    browse_tournaments(
                        category,
                        request.search.clone(),
                        request.page,
                        request.format,
                        request.mode,
                    )
                    .await,
                )
            } else {
                None
            };
            (request, response)
        },
    );
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            let Some((request, Some(Ok(response)))) = resource.get() else {
                return;
            };
            if request == current.get_untracked() && request.page != response.page {
                navigate(
                    &browse_url(
                        &request.path,
                        &request.search,
                        response.page,
                        request.format,
                        request.mode,
                    ),
                    replace_navigation_options(),
                );
            }
        });
    }
    let refresh = Callback::new(move |()| revision.update(|value| *value = value.wrapping_add(1)));
    let websocket = expect_context::<WebsocketContext>();
    Effect::watch(
        move || websocket.message.get(),
        move |message, _, _| {
            let Some(ServerResult::Ok(message)) = message else {
                return;
            };
            let ServerMessage::Tournament(update) = message.as_ref() else {
                return;
            };
            let membership_changed = match update {
                TournamentUpdate::Joined(_) | TournamentUpdate::Left { .. } => matches!(
                    category,
                    TournamentCategory::Joined
                        | TournamentCategory::History
                        | TournamentCategory::Invitations
                ),
                TournamentUpdate::OrganizerJoined(_) | TournamentUpdate::OrganizerLeft(_, _) => {
                    matches!(
                        category,
                        TournamentCategory::Organizing
                            | TournamentCategory::History
                            | TournamentCategory::Invitations
                    )
                }
                TournamentUpdate::Invited(_)
                | TournamentUpdate::Declined(_)
                | TournamentUpdate::Uninvited(_)
                | TournamentUpdate::OrganizerInvited(_)
                | TournamentUpdate::OrganizerUninvited(_)
                | TournamentUpdate::OrganizerInvitationsClosed(_) => {
                    category == TournamentCategory::Invitations
                }
                _ => false,
            };
            if membership_changed
                && current.get_untracked().path.trim_end_matches('/') == category_path(category)
            {
                refresh.run(());
            }
        },
        false,
    );

    let on_search = Callback::new({
        let navigate = navigate.clone();
        move |search: String| {
            let request = current.get_untracked();
            navigate(
                &browse_url(&request.path, &search, 1, request.format, request.mode),
                replace_navigation_options(),
            );
        }
    });
    let on_format = Callback::new({
        let navigate = navigate.clone();
        move |format| {
            let request = current.get_untracked();
            navigate(
                &browse_url(
                    &request.path,
                    &request.search,
                    1,
                    format,
                    if request.format == Some(TournamentFamily::Arena)
                        && format != Some(TournamentFamily::Arena)
                    {
                        TournamentTimeFilter::Both
                    } else {
                        request.mode
                    },
                ),
                replace_navigation_options(),
            );
        }
    });
    let on_mode = Callback::new({
        let navigate = navigate.clone();
        move |mode| {
            let request = current.get_untracked();
            navigate(
                &browse_url(&request.path, &request.search, 1, request.format, mode),
                replace_navigation_options(),
            );
        }
    });
    let on_page_change = Callback::new(move |page| {
        let request = current.get_untracked();
        navigate(
            &browse_url(
                &request.path,
                &request.search,
                page,
                request.format,
                request.mode,
            ),
            NavigateOptions {
                replace: false,
                scroll: true,
                ..Default::default()
            },
        );
    });
    let results = move || {
        let (request, response) = resource.get()?;
        if request != current.get() {
            return None;
        }
        response.map(|result| {
            result.map(|response| {
                view! { <TournamentBrowseResult response request current category on_page_change /> }
            })
        })
    };

    view! {
        <Panel body_class="space-y-3">
            <div class="flex flex-wrap gap-2 items-center">
                <TournamentSearch search on_search />
                // TODO: i18n once copy is approved.
                <label class="min-w-0 grow sm:grow-0">
                    <span class="sr-only">"Format"</span>
                    <select
                        class="ui-field-select"
                        prop:value=move || format.get().map_or("", |value| value.as_str())
                        on:change=move |event| {
                            on_format.run(event_target_value(&event).parse().ok())
                        }
                    >
                        <option value="">"All formats"</option>
                        <option value="swiss">"Swiss"</option>
                        <option value="round_robin">"Round robin"</option>
                        <option value="arena">"Arena"</option>
                        <option value="elimination">"Elimination"</option>
                    </select>
                </label>
                <Show when=move || format.get() != Some(TournamentFamily::Arena)>
                    // TODO: i18n once copy is approved.
                    <label class="min-w-0 grow sm:grow-0">
                        <span class="sr-only">"Time mode"</span>
                        <select
                            class="ui-field-select"
                            prop:value=move || mode.get().as_str()
                            on:change=move |event| {
                                on_mode.run(event_target_value(&event).parse().unwrap_or_default())
                            }
                        >
                            <option value="both">"Both"</option>
                            <option value="live">"Live"</option>
                            <option value="correspondence">"Correspondence"</option>
                        </select>
                    </label>
                </Show>
                <button
                    type="button"
                    class="min-h-10 ui-button ui-button-secondary ui-button-sm"
                    on:click=move |_| refresh.run(())
                >
                    // TODO: i18n once copy is approved.
                    "Refresh"
                </button>
            </div>
            // TODO: i18n once copy is approved.
            <p class="text-xs text-gray-500 dark:text-gray-400">"Times shown in your local time"</p>
            <Suspense fallback=move || {
                view! {
                    <div role="status" aria-busy="true">
                        <EmptyState title=move || t_string!(i18n, tournaments.list.loading) />
                    </div>
                }
            }>
                <ErrorBoundary fallback=move |_| {
                    view! {
                        <EmptyState title=move || t_string!(i18n, tournaments.list.error) />
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-sm"
                            on:click=move |_| refresh.run(())
                        >
                            // TODO: i18n once copy is approved.
                            "Retry"
                        </button>
                    }
                }>{results}</ErrorBoundary>
            </Suspense>
        </Panel>
    }
}

#[component]
fn TournamentBrowseResult(
    response: TournamentBrowsePage,
    request: BrowseRequest,
    current: Memo<BrowseRequest>,
    category: TournamentCategory,
    on_page_change: Callback<usize>,
) -> impl IntoView {
    if response.tournaments.is_empty() {
        let i18n = use_i18n();
        let title = move || {
            if request.search.is_empty()
                && request.format.is_none()
                && request.mode == TournamentTimeFilter::Both
            {
                empty_list_title(i18n, category)
            } else {
                t_string!(i18n, tournaments.list.no_results).to_string()
            }
        };
        Either::Left(view! { <EmptyState title /> })
    } else {
        Either::Right(
            view! { <TournamentBrowseSnapshot response request current category on_page_change /> },
        )
    }
}

#[component]
fn TournamentSearch(search: Signal<String>, on_search: Callback<String>) -> impl IntoView {
    let i18n = use_i18n();
    let draft = RwSignal::new(search.get_untracked());
    let last_sent = RwSignal::new(search.get_untracked());
    let send_search = Callback::new(move |value: String| {
        last_sent.set(value.trim().to_owned());
        on_search.run(value);
    });
    let UseTimeoutFnReturn { start, stop, .. } =
        use_timeout_fn(move |value: String| send_search.run(value), 250.0);
    let cancel_search = Callback::new(move |()| stop());
    Effect::new(move || {
        let query = search.get();
        // A navigation we dispatched may complete after more typing; preserve that newer draft.
        if query != last_sent.get_untracked() {
            draft.set(query.clone());
            last_sent.set(query);
            cancel_search.run(());
        }
    });
    let on_input = move |event| {
        let value = event_target_value(&event);
        draft.set(value.clone());
        if value.is_empty() {
            cancel_search.run(());
            send_search.run(value);
        } else {
            start(value);
        }
    };
    view! {
        <label class="w-full min-w-0 sm:flex-1 sm:w-auto">
            <span class="sr-only">{t!(i18n, tournaments.list.search)}</span>
            <input
                type="search"
                class="ui-field-input"
                placeholder=move || t_string!(i18n, tournaments.list.search_placeholder)
                on:input=on_input
                on:keydown=move |event| {
                    if event.key() == "Enter" {
                        event.prevent_default();
                        cancel_search.run(());
                        send_search.run(draft.get_untracked());
                    }
                }
                prop:value=draft
            />
        </label>
    }
}
