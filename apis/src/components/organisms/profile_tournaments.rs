use crate::{
    components::molecules::{
        empty_state::EmptyState,
        tournament_pagination::TournamentPagination,
        tournament_row::{TournamentRow, ViewerStatusStyle},
    },
    functions::tournaments::get_profile_tournaments,
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{AuthContext, AuthIdentity},
    responses::{
        ProfileTournamentCard,
        ProfileTournamentResult,
        ProfileTournamentsPage,
        TournamentCardProgress,
        TournamentCategory,
        TOURNAMENT_BROWSE_PAGE_SIZE,
    },
};
use chrono::{DateTime, Utc};
use leptos::{either::Either, prelude::*};
use leptos_icons::Icon;
use leptos_router::{
    hooks::{use_location, use_navigate, use_params},
    params::Params,
    NavigateOptions,
};

const CURRENT_ROWS_BEFORE_SHOW_ALL: usize = 10;
const NOTICE_LINK_CLASS: &str = "flex gap-2 items-center ui-notice no-link-style hover:bg-pillbug-teal/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-pillbug-teal";

#[derive(Params, PartialEq, Eq)]
struct UsernameParams {
    username: String,
}

fn tournaments_url(username: &str, page: usize) -> String {
    if page > 1 {
        format!("/@/{username}/tournaments?page={page}")
    } else {
        format!("/@/{username}/tournaments")
    }
}

fn parsed_page(raw: Option<String>) -> usize {
    raw.and_then(|page| page.parse::<usize>().ok())
        .filter(|page| *page > 0)
        .unwrap_or(1)
}

fn role_label(card: &ProfileTournamentCard) -> Option<String> {
    let i18n = use_i18n();
    let (playing, organizing) = match card.card.summary.progress {
        TournamentCardProgress::Entrants { .. } => (
            t_string!(i18n, common.joined).to_string(),
            t_string!(i18n, common.organizing).to_string(),
        ),
        TournamentCardProgress::Complete => (
            t_string!(i18n, profile.tournaments.played).to_string(),
            t_string!(i18n, profile.tournaments.organized).to_string(),
        ),
        _ => (
            t_string!(i18n, common.playing).to_string(),
            t_string!(i18n, common.organizing).to_string(),
        ),
    };
    match (card.role.playing, card.role.organizing) {
        (true, true) => Some(format!("{playing} · {organizing}")),
        (true, false) => Some(playing),
        (false, true) => Some(organizing),
        (false, false) => None,
    }
}

fn result_label(card: &ProfileTournamentCard) -> Option<String> {
    let i18n = use_i18n();
    match card.result? {
        ProfileTournamentResult::Ranked { rank, total } => Some(
            t_string!(
                i18n,
                profile.tournaments.rank_of_total,
                rank = (rank),
                total = (total)
            )
            .to_string(),
        ),
        ProfileTournamentResult::Withdrawn => Some(t_string!(i18n, common.withdrawn).to_string()),
    }
}

#[component]
pub fn ProfileTournaments() -> impl IntoView {
    let i18n = use_i18n();
    let params = use_params::<UsernameParams>();
    let username = Signal::derive(move || {
        params.with(|p| p.as_ref().map(|p| p.username.clone()).unwrap_or_default())
    });
    let location = use_location();
    let navigate = use_navigate();
    let auth = expect_context::<AuthContext>();
    let viewer = Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let is_owner = Signal::derive(move || {
        auth.user.with(|account| {
            account.as_ref().is_some_and(|account| {
                account.username.to_lowercase() == username.get().to_lowercase()
            })
        })
    });
    let page = Memo::new(move |_| parsed_page(location.query.get().get("page")));
    let resource = Resource::new(
        move || (username.get(), page.get(), viewer.get()),
        move |(username, page, _)| get_profile_tournaments(username, page),
    );
    let on_page_change = Callback::new(move |page: usize| {
        navigate(
            &tournaments_url(&username.get_untracked(), page),
            NavigateOptions {
                scroll: false,
                ..Default::default()
            },
        );
    });
    let now = use_ticking_now();

    view! {
        <div class="overflow-y-auto flex-1 min-h-0">
            <div class="mx-auto w-full max-w-5xl">
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
                        }
                    }>
                        {move || {
                            let is_owner = is_owner.get();
                            resource
                                .get()
                                .map(|result| {
                                    result
                                        .map(|page| {
                                            view! {
                                                <ProfileTournamentsContent
                                                    page
                                                    is_owner
                                                    now
                                                    on_page_change
                                                />
                                            }
                                        })
                                })
                        }}
                    </ErrorBoundary>
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn ProfileTournamentsContent(
    page: ProfileTournamentsPage,
    is_owner: bool,
    now: Signal<DateTime<Utc>>,
    on_page_change: Callback<usize>,
) -> impl IntoView {
    let i18n = use_i18n();
    let invitations = page.invitations.unwrap_or_default();
    let pending = i64::try_from(invitations.len()).unwrap_or(i64::MAX);
    let single_invitation =
        StoredValue::new((invitations.len() == 1).then(|| invitations[0].clone()));
    // Keep comparisons outside attributes: leptosfmt strips braces needed around `>`.
    let has_pending = pending > 0;
    let current_page = page.page;
    let finished_total = page.finished_total as usize;
    let show_pagination = finished_total > TOURNAMENT_BROWSE_PAGE_SIZE;
    let no_tournaments = page.active.is_empty() && page.finished.is_empty();
    let show_current = is_owner || !page.active.is_empty();
    let show_past = !page.finished.is_empty();
    let organizes = is_owner && page.active.iter().any(|card| card.role.organizing);
    let active = StoredValue::new(page.active);
    let finished = StoredValue::new(page.finished);
    let viewer_status = if is_owner {
        ViewerStatusStyle::Owner
    } else {
        ViewerStatusStyle::Visitor
    };

    view! {
        <div class="flex flex-col gap-4 pt-1 pb-4">
            <Show when=move || has_pending>
                <a href="/tournaments/invitations" class=NOTICE_LINK_CLASS>
                    <Icon
                        icon=icondata_ai::AiMailOutlined
                        attr:class="shrink-0 size-4 text-pillbug-teal"
                    />
                    <span class="font-medium">
                        {match single_invitation.get_value() {
                            Some(name) => {
                                Either::Left(
                                    t!(
                                        i18n, profile.tournaments.invited_to_name, name = move || name.clone()
                                    ),
                                )
                            }
                            None => {
                                Either::Right(
                                    t!(
                                        i18n, profile.tournaments.pending_invitations, count = move || pending
                                    ),
                                )
                            }
                        }}
                    </span>
                    <Icon
                        icon=icondata_lu::LuChevronRight
                        attr:class="ml-auto shrink-0 size-4 text-pillbug-teal"
                    />
                </a>
            </Show>
            {if no_tournaments {
                Either::Left(
                    view! {
                        <EmptyState title=move || t_string!(i18n, profile.tournaments.none)>
                            <EmptyActions is_owner />
                        </EmptyState>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <p class="text-xs text-gray-500 dark:text-gray-400">
                            {t!(i18n, tournaments.list.times_shown_in_your_local_time)}
                        </p>
                        <Show when=move || show_current>
                            <TournamentGroup
                                title=t_string!(i18n, profile.tournaments.current).to_string()
                                empty=t_string!(i18n, profile.tournaments.no_current).to_string()
                                cards=active.get_value()
                                category=TournamentCategory::Joined
                                viewer_status
                                is_owner
                                manage_link=organizes
                                collapse_after=CURRENT_ROWS_BEFORE_SHOW_ALL
                                now
                            />
                        </Show>
                        <Show when=move || show_past>
                            <TournamentGroup
                                title=t_string!(i18n, profile.tournaments.past).to_string()
                                empty=t_string!(i18n, profile.tournaments.no_past).to_string()
                                cards=finished.get_value()
                                category=TournamentCategory::History
                                viewer_status
                                is_owner
                                now
                            />
                        </Show>
                        <Show when=move || show_pagination>
                            <TournamentPagination
                                page=current_page
                                total=finished_total
                                on_page_change
                            />
                        </Show>
                    },
                )
            }}
        </div>
    }
}

#[component]
fn EmptyActions(is_owner: bool) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <a class="ui-button ui-button-secondary ui-button-sm no-link-style" href="/tournaments">
            {t!(i18n, tournaments.view_list)}
        </a>
        <Show when=move || is_owner>
            <a
                class="ui-button ui-button-secondary ui-button-sm no-link-style"
                href="/tournaments/create"
            >
                {t!(i18n, tournaments.create)}
            </a>
        </Show>
    }
}

#[component]
fn TournamentGroup(
    title: String,
    empty: String,
    cards: Vec<ProfileTournamentCard>,
    category: TournamentCategory,
    viewer_status: ViewerStatusStyle,
    is_owner: bool,
    #[prop(optional)] manage_link: bool,
    #[prop(optional)] collapse_after: Option<usize>,
    now: Signal<DateTime<Utc>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let on_invitation = Callback::new(|_| ());
    let total = cards.len();
    let show_all = RwSignal::new(false);
    let collapsible = collapse_after.is_some_and(|limit| total > limit);
    let visible_count = move || {
        if collapsible && !show_all.get() {
            collapse_after.unwrap_or(total)
        } else {
            total
        }
    };
    let rows = StoredValue::new(cards);
    view! {
        <section class="flex flex-col gap-2">
            <div class="flex flex-wrap gap-y-1 gap-x-4 justify-between items-baseline">
                <h2 class="text-base font-bold text-gray-900 dark:text-gray-100">{title}</h2>
                <Show when=move || manage_link>
                    <a
                        class="text-sm font-medium hover:underline text-pillbug-teal"
                        href="/tournaments/organize"
                    >
                        {t!(i18n, profile.tournaments.manage_organizing)}
                    </a>
                </Show>
            </div>
            {if total == 0 {
                Either::Left(
                    view! {
                        <EmptyState title=empty>
                            <EmptyActions is_owner />
                        </EmptyState>
                    },
                )
            } else {
                Either::Right(
                    view! {
                        <div class="overflow-hidden rounded-lg border divide-y border-black/10 divide-black/10 dark:border-white/10 dark:divide-white/10">
                            {move || {
                                rows.with_value(|cards| {
                                    cards
                                        .iter()
                                        .take(visible_count())
                                        .map(|card| {
                                            let role_label = role_label(card);
                                            let result_label = result_label(card);
                                            let tournament = Signal::stored(card.card.clone());
                                            view! {
                                                <TournamentRow
                                                    tournament
                                                    now
                                                    unavailable=Signal::stored(false)
                                                    invitation_pending=Signal::stored(false)
                                                    on_invitation
                                                    category
                                                    viewer_status
                                                    role_label
                                                    result_label
                                                />
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                })
                            }}
                        </div>
                        <Show when=move || collapsible>
                            <button
                                type="button"
                                class="self-center ui-button ui-button-secondary ui-button-sm"
                                on:click=move |_| show_all.update(|expanded| *expanded = !*expanded)
                            >
                                {move || {
                                    if show_all.get() {
                                        Either::Left(t!(i18n, profile.tournaments.show_fewer))
                                    } else {
                                        Either::Right(
                                            t!(
                                                i18n, profile.tournaments.show_all, total = move || total
                                            ),
                                        )
                                    }
                                }}
                            </button>
                        </Show>
                    },
                )
            }}
        </section>
    }
}
