use crate::{
    components::{
        layouts::{
            page_header::PageHeader,
            page_shell::{PageShell, PageShellVariant},
        },
        molecules::{
            empty_state::EmptyState,
            pagination_controls::PaginationControls,
            panel::Panel,
            tournament_row::TournamentRow,
        },
    },
    functions::tournaments::browse_tournaments,
    i18n::*,
    providers::{websocket::WebsocketContext, AuthContext, AuthIdentity, UpdateNotifier},
    responses::{TournamentBrowsePage, TournamentCategory, TOURNAMENT_BROWSE_PAGE_SIZE},
};
use leptos::{leptos_dom::helpers::debounce, prelude::*};
use leptos_router::{
    components::{Redirect, A},
    hooks::{use_location, use_navigate},
    location::{State, Url},
    NavigateOptions,
};
use std::time::Duration;

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

fn browse_url(path: &str, search: &str, page: usize) -> String {
    let search = search.trim();
    let mut query = Vec::new();
    if !search.is_empty() {
        query.push(format!("q={}", Url::escape(search)));
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

    view! {
        <PageShell variant=PageShellVariant::Dashboard>
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
                        href="/tournaments"
                        attr:class=move || tournament_tab_class(path_is("/tournaments"))
                    >
                        "Upcoming"
                    </A>
                    // TODO: i18n once copy is approved.
                    <A
                        href="/tournaments/in-progress"
                        attr:class=move || tournament_tab_class(path_is("/tournaments/in-progress"))
                    >
                        "In progress"
                    </A>
                    // TODO: i18n once copy is approved.
                    <A
                        href="/tournaments/finished"
                        attr:class=move || tournament_tab_class(path_is("/tournaments/finished"))
                    >
                        "Finished"
                    </A>
                    <Show when=logged_in>
                        // TODO: i18n once copy is approved.
                        <A
                            href="/tournaments/mine/joined"
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
                            href="/tournaments/mine/joined"
                            attr:class=move || mine_tab_class(path_is("/tournaments/mine/joined"))
                        >
                            "Joined"
                        </A>
                        // TODO: i18n once copy is approved.
                        <A
                            href="/tournaments/mine/organizing"
                            attr:class=move || mine_tab_class(
                                path_is("/tournaments/mine/organizing"),
                            )
                        >
                            "Organizing"
                        </A>
                        // TODO: i18n once copy is approved.
                        <A
                            href="/tournaments/mine/invitations"
                            attr:class=move || mine_tab_class(
                                path_is("/tournaments/mine/invitations"),
                            )
                        >
                            "Invitations"
                        </A>
                        // TODO: i18n once copy is approved.
                        <A
                            href="/tournaments/mine/history"
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
    view! {
        <Redirect
            path="/tournaments/mine/joined"
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
    // Keep filtering and canonical navigation on the same URL snapshot. An
    // outlet query can lag the global URL while the previous browse is pending.
    let queries = location.query;
    let navigate = use_navigate();
    let update = expect_context::<UpdateNotifier>().tournament_catalog_update;
    let websocket = expect_context::<WebsocketContext>();
    let auth = expect_context::<AuthContext>();
    let search = Memo::new(move |_| queries.get().get("q").unwrap_or_default());
    let requested_page = Memo::new(move |_| parsed_page(queries.get().get("page")));

    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            let path = location.pathname.get();
            let canonical = browse_url(&path, &search.get(), requested_page.get());
            let raw_search = location.search.get();
            let current = if raw_search.is_empty() {
                path
            } else {
                format!("{path}?{raw_search}")
            };
            if current != canonical {
                navigate(&canonical, replace_navigation_options());
            }
        });
    }

    let resource = Resource::new(
        move || {
            (
                search.get(),
                requested_page.get(),
                update.get(),
                websocket.ready_state.get(),
                auth.identity.get(),
            )
        },
        move |(search, page, _, _, _)| async move {
            let response = browse_tournaments(category, search.clone(), page).await;
            (search, page, response)
        },
    );
    let total = Signal::derive(move || {
        resource
            .get()
            .and_then(|(_, _, response)| response.ok())
            .map_or(0, |response| response.total as usize)
    });
    let response_page = Signal::derive(move || {
        resource
            .get()
            .and_then(|(_, _, response)| response.ok())
            .map_or_else(|| requested_page.get_untracked(), |response| response.page)
    });
    {
        let navigate = navigate.clone();
        Effect::new(move |_| {
            let Some((response_search, request_page, Ok(response))) = resource.get() else {
                return;
            };
            // Only a completed response for the current request may clamp the URL.
            // Tracking URL changes here would inspect the previous response while
            // its replacement is pending and interfere with the resource's update.
            if response_search == search.get_untracked()
                && request_page == requested_page.get_untracked()
                && response.page != request_page
            {
                navigate(
                    &browse_url(
                        &location.pathname.get_untracked(),
                        &response_search,
                        response.page,
                    ),
                    replace_navigation_options(),
                );
            }
        });
    }
    let on_search = Callback::new({
        let navigate = navigate.clone();
        move |next_search: String| {
            navigate(
                &browse_url(&location.pathname.get_untracked(), &next_search, 1),
                replace_navigation_options(),
            );
        }
    });
    let on_page_change = Callback::new(move |next_page| {
        navigate(
            &browse_url(
                &location.pathname.get_untracked(),
                &search.get_untracked(),
                next_page,
            ),
            NavigateOptions {
                resolve: true,
                replace: false,
                scroll: true,
                state: State::new(None),
            },
        );
    });

    view! {
        <TournamentSearch search=search.into() on_search />
        <Transition fallback=move || {
            view! { <EmptyState title=move || t_string!(i18n, tournaments.list.loading) /> }
        }>
            {move || {
                resource
                    .get()
                    .map(|(response_search, _, response)| match response {
                        Ok(TournamentBrowsePage { tournaments, .. }) if tournaments.is_empty() => {
                            let title = if response_search.trim().is_empty() {
                                empty_list_title(i18n, category)
                            } else {
                                t_string!(i18n, tournaments.list.no_results).to_string()
                            };
                            view! { <EmptyState title=title /> }.into_any()
                        }
                        Ok(TournamentBrowsePage { tournaments, .. }) => {
                            view! {
                                <div class="space-y-3">
                                    <div class="flex flex-col gap-2">
                                        // TODO: i18n once copy is approved.
                                        <div
                                            class="hidden gap-x-3 items-center py-2 px-4 text-xs font-semibold text-gray-600 lg:grid dark:text-gray-300 lg:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)_minmax(0,0.65fr)_minmax(0,0.9fr)_minmax(0,1.1fr)_minmax(0,1.1fr)]"
                                            aria-hidden="true"
                                        >
                                            <span>"Tournament"</span>
                                            <span>"Format and clock"</span>
                                            <span>"Field"</span>
                                            <span>"Progress"</span>
                                            <span>"Date"</span>
                                            <span>"Status / actions"</span>
                                        </div>
                                        {tournaments
                                            .into_iter()
                                            .map(|tournament| {
                                                view! { <TournamentRow tournament category /> }
                                            })
                                            .collect_view()}
                                    </div>
                                    <PaginationControls
                                        page=response_page
                                        total
                                        page_size=TOURNAMENT_BROWSE_PAGE_SIZE
                                        on_page_change
                                    />
                                </div>
                            }
                                .into_any()
                        }
                        Err(_) => {
                            view! {
                                <EmptyState title=move || t_string!(i18n, tournaments.list.error) />
                            }
                                .into_any()
                        }
                    })
            }}
        </Transition>
    }
}

#[component]
fn TournamentSearch(search: Signal<String>, on_search: Callback<String>) -> impl IntoView {
    let i18n = use_i18n();
    let draft = RwSignal::new(search.get_untracked());
    let last_sent = RwSignal::new(search.get_untracked());
    let send_search = Callback::new(move |value: String| {
        last_sent.set(value.clone());
        on_search.run(value);
    });
    let generation = ArcRwSignal::new(0_u64);
    let mounted = ArcRwSignal::new(true);
    let mounted_for_cleanup = mounted.clone();
    on_cleanup(move || mounted_for_cleanup.set(false));
    let pathname = use_location().pathname;
    let current_path = ArcRwSignal::new(pathname.get_untracked());
    let path_for_effect = current_path.clone();
    Effect::new(move || path_for_effect.set(pathname.get()));
    let generation_for_sync = generation.clone();
    Effect::new(move || {
        let query = search.get();
        // A navigation we dispatched may complete after more typing; preserve that newer draft.
        if query != last_sent.get_untracked() {
            draft.set(query.clone());
            last_sent.set(query);
            generation_for_sync.update(|value| *value += 1);
        }
    });
    let generation_for_timer = generation.clone();
    let mut delayed_search = debounce(
        Duration::from_millis(250),
        move |(value, request, path): (String, u64, String)| {
            if mounted.get_untracked()
                && generation_for_timer.get_untracked() == request
                && current_path.get_untracked() == path
            {
                send_search.run(value);
            }
        },
    );
    let generation_for_input = generation.clone();
    let on_input = move |event| {
        let value = event_target_value(&event);
        draft.set(value.clone());
        generation_for_input.update(|generation| *generation += 1);
        if value.is_empty() {
            send_search.run(value);
        } else {
            delayed_search((
                value,
                generation_for_input.get_untracked(),
                pathname.get_untracked(),
            ));
        }
    };
    view! {
        <Panel body_class="space-y-2" class="w-full">
            <label class="flex flex-col gap-1.5">
                <span class="ui-field-label">{t!(i18n, tournaments.list.search)}</span>
                <input
                    type="search"
                    class="ui-field-input"
                    placeholder=move || t_string!(i18n, tournaments.list.search_placeholder)
                    on:input=on_input
                    on:keydown=move |event| {
                        if event.key() == "Enter" {
                            event.prevent_default();
                            generation.update(|value| *value += 1);
                            send_search.run(draft.get_untracked());
                        }
                    }
                    prop:value=draft
                />
            </label>
        </Panel>
    }
}
