use crate::{
    common::{
        format_tournament_datetime,
        markdown_to_html,
        tournament_format_label,
        tournament_path_matches,
        with_class,
        ClientRequest,
        TournamentAction,
        TournamentStartIntent,
    },
    components::{
        layouts::page_shell::{PageShell, PageShellVariant},
        molecules::{modal::Modal, panel::Panel, time_row::TimeRow, user_identity::UserIdentity},
        organisms::{
            bracket::{Bracket, BracketPlacementPreview, BracketPosition},
            chat::ResolvedChatWindow,
            tournament_admin::TournamentAdminControls,
            tournament_closeout::TournamentCloseout,
            tournament_detailed_standings::TournamentDetailedStandings,
            tournament_encounters::{RoundRobinCrosstable, SwissRoundBrowser},
            tournament_explanations::{
                StandingsCriteriaRules,
                TournamentFormatConfigurationDetails,
                TournamentFormatFaq,
            },
            tournament_inspector::{TournamentOverviewInspector, TournamentSelection},
            tournament_organizers::TournamentOrganizers,
            tournament_overview_rail::TournamentOverviewRail,
            tournament_overview_standings::TournamentOverviewStandings,
            tournament_participant_action::TournamentParticipantAction,
            tournament_slots::{
                participant_schedule::participant_schedule_opponents_needing_time,
                ParticipantScheduleIndex,
                ParticipantScheduleLayout,
                ParticipantScheduleOpponent,
                TournamentOrganizerIndex,
                TournamentOrganizerLayout,
                TournamentOrganizerMatch,
            },
            tournament_withdrawal::TournamentWithdrawal,
        },
    },
    functions::{
        schedules::get_tournament_schedules,
        tournaments::{get_complete, UpdateDescription},
    },
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{
        schedules::{ScheduleLoadKey, SchedulesContext},
        websocket::{ConnectionReadyState, WebsocketContext},
        ActiveTournamentState,
        ApiRequestsProvider,
        ArenaStateStoreFields,
        AuthContext,
        AuthIdentity,
        EliminationStateStoreFields,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommonStoreFields,
        TournamentFormatStore,
        TournamentScheduleState,
        TournamentState,
    },
    responses::{
        TournamentLifecycleDetails,
        TournamentMemberships,
        TournamentPatch,
        TournamentResponse,
    },
};
use chrono::{DateTime, Duration, Utc};
use leptos::{
    ev::pagehide,
    html::{Aside, Details, Dialog, Div, Summary},
    prelude::*,
};
use leptos_icons::Icon;
use leptos_router::{
    any_nested_route::IntoAnyNestedRoute,
    components::{Outlet, ParentRoute, ProtectedParentRoute, ProtectedRoute, Redirect, Route, A},
    hooks::{query_signal_with_options, use_location, use_params_map},
    path,
    NavigateOptions,
};
use leptos_use::{use_element_size, use_event_listener, use_media_query, use_window};
use reactive_stores::{ArcField, Store};
use shared_types::{
    tournament::{BotAdmission, Format, FormatConfig},
    ConversationKey,
    GameSpeed,
    TimeMode,
    TournamentId,
    TournamentStartSetup,
    TournamentStatus,
};
use std::collections::HashMap;
use uuid::Uuid;

fn tournament_response_matches_route(
    route_id: Option<&TournamentId>,
    loaded_id: &TournamentId,
) -> bool {
    route_id == Some(loaded_id)
}

fn should_retry_failed_initial_load(
    load_failed: bool,
    ready_state: ConnectionReadyState,
    previous: Option<ConnectionReadyState>,
) -> bool {
    load_failed
        && ready_state == ConnectionReadyState::Open
        && previous.is_some_and(|previous| previous != ConnectionReadyState::Open)
}

#[cfg(feature = "hydrate")]
fn confirm_tournament_deletion(message: &str) -> bool {
    window().confirm_with_message(message).unwrap_or(false)
}

#[cfg(not(feature = "hydrate"))]
fn confirm_tournament_deletion(_message: &str) -> bool {
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PreStartBoundary {
    Open,
    WaitingForScheduledWorker,
    Closed,
}

fn pre_start_boundary(
    is_open: bool,
    format: Format,
    starts_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> PreStartBoundary {
    if !is_open {
        return PreStartBoundary::Closed;
    }
    match starts_at {
        None => PreStartBoundary::Open,
        Some(starts_at) if now < starts_at => PreStartBoundary::Open,
        Some(_) if format == Format::Arena => PreStartBoundary::Closed,
        Some(_) => PreStartBoundary::WaitingForScheduledWorker,
    }
}

#[component]
pub fn Tournament() -> impl IntoView {
    let params = use_params_map();
    let tournament_id = Memo::new(move |_| {
        params
            .get()
            .get("nanoid")
            .map(|id| TournamentId(id.to_string()))
    });

    view! {
        <PageShell variant=PageShellVariant::Tournament>
            <div class="flex flex-col gap-3 ui-wide-canvas">
                <For
                    each=move || tournament_id.get().into_iter()
                    key=|id| id.clone()
                    let:tournament_id
                >
                    <TournamentRoute tournament_id />
                </For>
            </div>
        </PageShell>
    }
}

#[component]
fn TournamentRoute(tournament_id: TournamentId) -> impl IntoView {
    let i18n = use_i18n();
    let websocket = expect_context::<WebsocketContext>();
    let requested_id = tournament_id.clone();
    let tournament = Resource::new(move || tournament_id.clone(), get_complete);

    let previous_ready_state = StoredValue::new(None::<ConnectionReadyState>);
    Effect::new(move |_| {
        let ready_state = websocket.ready_state.get();
        let load_failed = tournament.get().is_some_and(|response| response.is_err());
        if should_retry_failed_initial_load(
            load_failed,
            ready_state,
            previous_ready_state.get_value(),
        ) {
            tournament.refetch();
        }
        previous_ready_state.set_value(Some(ready_state));
    });
    view! {
        <Transition fallback=move || {
            view! {
                <Panel>
                    <p class="text-sm text-gray-600 dark:text-gray-300">
                        {t!(i18n, tournaments.detail.loading)}
                    </p>
                </Panel>
            }
        }>
            {move || {
                tournament
                    .get()
                    .map(|response| match response {
                        Ok(
                            Some(tournament),
                        ) if tournament_response_matches_route(
                            Some(&requested_id),
                            &tournament.tournament_id,
                        ) => view! { <LoadedTournament initial_tournament=tournament /> }.into_any(),
                        Ok(None) => {
                            view! {
                                <Panel>
                                    <p class="text-sm text-gray-600 dark:text-gray-300">
                                        {t!(i18n, tournaments.not_found)}
                                    </p>
                                </Panel>
                            }
                                .into_any()
                        }
                        Ok(Some(_)) | Err(_) => {
                            view! {
                                <Panel>
                                    <div class="flex flex-col gap-3 items-start">
                                        <p class="text-sm text-gray-600 dark:text-gray-300">
                                            {t!(i18n, tournaments.unavailable)}
                                        </p>
                                        <button
                                            type="button"
                                            class="ui-button ui-button-secondary ui-button-sm"
                                            on:click=move |_| tournament.refetch()
                                        >
                                            {t!(i18n, messages.chat.retry)}
                                        </button>
                                    </div>
                                </Panel>
                            }
                                .into_any()
                        }
                    })
            }}
        </Transition>
    }
}

fn use_tournament_live_updates(tournament: TournamentState) -> RwSignal<u64> {
    let websocket = expect_context::<WebsocketContext>();
    let active_tournament = expect_context::<ActiveTournamentState>();
    let registration = active_tournament.register(tournament);
    let tournament_id = StoredValue::new(tournament.tournament_id());
    let reconnect_generation = RwSignal::new(0_u64);
    let refresh = Action::new(move |_: &()| {
        let requested_id = tournament_id.get_value();
        async move {
            let response = get_complete(requested_id.clone()).await;
            (requested_id, response)
        }
    });

    Effect::new(move |_| {
        let Some((loaded_id, Ok(Some(response)))) = refresh.value().get() else {
            return;
        };
        if loaded_id == tournament_id.get_value()
            && response.tournament_id == tournament_id.get_value()
        {
            tournament.apply_snapshot(response);
        }
    });

    let has_watched_open_connection = StoredValue::new(false);
    let ready_state_websocket = websocket.clone();
    let ready_websocket = websocket.clone();
    Effect::watch(
        move || ready_state_websocket.ready_state.get(),
        move |ready_state, previous, _| {
            let connection_opened = *ready_state == ConnectionReadyState::Open
                && previous.is_none_or(|previous| *previous != ConnectionReadyState::Open);
            if !connection_opened {
                return;
            }

            ready_websocket.send(&ClientRequest::TournamentWatch(tournament_id.get_value()));
            if has_watched_open_connection.get_value() {
                reconnect_generation.update(|generation| {
                    *generation = generation.wrapping_add(1);
                });
                refresh.dispatch(());
            } else {
                has_watched_open_connection.set_value(true);
            }
        },
        true,
    );

    let wake_state_websocket = websocket.clone();
    let wake_websocket = websocket.clone();
    Effect::watch(
        move || wake_state_websocket.wake_resync_epoch.get(),
        move |wake_epoch, previous, _| {
            let woke = previous.is_some_and(|previous| previous != wake_epoch);
            if woke && wake_websocket.ready_state.get_untracked() == ConnectionReadyState::Open {
                wake_websocket.send(&ClientRequest::TournamentWatch(tournament_id.get_value()));
                reconnect_generation.update(|generation| {
                    *generation = generation.wrapping_add(1);
                });
                refresh.dispatch(());
            }
        },
        false,
    );

    let cleanup_websocket = websocket;
    on_cleanup(move || {
        let tournament_id = tournament_id.get_value();
        if active_tournament.cleanup_should_unwatch(registration, &tournament_id) {
            cleanup_websocket.send(&ClientRequest::TournamentUnwatch(tournament_id));
        }
    });
    reconnect_generation
}

#[component]
fn LoadedTournament(initial_tournament: TournamentResponse) -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    let identity = auth.identity;
    let admin = auth.admin;
    let tournament_id = StoredValue::new(initial_tournament.tournament_id.clone());
    let Ok(tournament) = TournamentState::new(initial_tournament) else {
        return view! {
            <Panel>
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    {t!(i18n, tournaments.unavailable)}
                </p>
            </Panel>
        }
        .into_any();
    };
    let reconnect_generation = use_tournament_live_updates(tournament);

    let user_id = Memo::new(move |_| identity.get().and_then(AuthIdentity::user_id));
    let can_view_all_schedules = Memo::new(move |_| {
        let Some(user_id) = user_id.get() else {
            return false;
        };
        admin.get().unwrap_or(false)
            || tournament
                .common
                .memberships()
                .get()
                .organizers
                .iter()
                .any(|organizer| organizer.uid == user_id)
    });
    let schedules_context = expect_context::<SchedulesContext>();
    let schedules: TournamentScheduleState = Store::new(HashMap::new());
    let schedules_ready = RwSignal::new(false);
    let schedule_owner_id = tournament_id.get_value();
    let schedule_registration =
        schedules_context.set_tournament(schedule_owner_id.clone(), schedules);
    on_cleanup(move || {
        let _ = schedules_context.clear_tournament(schedule_registration);
    });
    let schedule_load_key = Memo::new(move |_| {
        user_id.get().map(|user_id| ScheduleLoadKey {
            tournament_id: tournament_id.get_value(),
            user_id,
            load_epoch: reconnect_generation.get(),
            can_view_all_schedules: can_view_all_schedules.get(),
            registration: schedule_registration,
        })
    });
    let schedule_resource = Resource::new(
        move || {
            schedules_ready.set(false);
            let requested_id = tournament_id.get_value();
            let Some(key) = schedule_load_key.get() else {
                schedules_context.clear_tournament_schedules(&requested_id, schedule_registration);
                return None;
            };
            schedules_context
                .begin_tournament_snapshot(&key)
                .then_some(key)
        },
        move |key| async move {
            let key = key?;
            let snapshot = get_tournament_schedules(key.tournament_id.clone()).await;
            Some((key, snapshot))
        },
    );
    Effect::new(move |_| match schedule_resource.get() {
        Some(None) => {
            schedules_context
                .clear_tournament_schedules(&tournament_id.get_value(), schedule_registration);
        }
        Some(Some((key, Ok(snapshot))))
            if schedule_load_key.get_untracked().as_ref() == Some(&key) =>
        {
            if schedules_context.tournament_snapshot_apply(&key, snapshot) {
                schedules_ready.set(true);
            }
        }
        Some(Some((key, Err(_)))) if schedule_load_key.get_untracked().as_ref() == Some(&key) => {
            schedules_context.finish_tournament_snapshot(&key);
        }
        None | Some(Some((_, Err(_)))) | Some(Some((_, Ok(_)))) => {}
    });

    view! { <TournamentSurface tournament schedules schedules_ready=schedules_ready.into() /> }
        .into_any()
}

#[derive(Clone, Copy)]
struct TournamentPageContext {
    tournament_id: StoredValue<TournamentId>,
    tournament: TournamentState,
    schedules: TournamentScheduleState,
    schedules_ready: Signal<bool>,
    user_id: Signal<Option<Uuid>>,
    user_joined: Signal<bool>,
    scheduling_available: Signal<bool>,
    organizer: Signal<bool>,
    auth_resolved: Signal<bool>,
    pre_start_changes_open: Signal<bool>,
    scheduled_start_pending: Signal<bool>,
    format: Format,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TournamentChildRoute {
    Overview,
    Results,
    Pairings,
    Standings,
    Bracket,
    Scheduling,
    Manage,
    ManageMatches,
}

fn child_route_availability(
    route: TournamentChildRoute,
    format: Format,
    status: TournamentStatus,
    scheduling_eligible: bool,
    organizer: bool,
    auth_resolved: bool,
) -> Option<bool> {
    if format == Format::Arena {
        return Some(route == TournamentChildRoute::Overview);
    }
    let started = status != TournamentStatus::NotStarted;
    let available = match route {
        TournamentChildRoute::Overview => true,
        TournamentChildRoute::Results => format == Format::RoundRobin && started,
        TournamentChildRoute::Pairings => {
            matches!(format, Format::Swiss | Format::DoubleSwiss) && started
        }
        TournamentChildRoute::Standings => {
            matches!(
                format,
                Format::RoundRobin | Format::Swiss | Format::DoubleSwiss
            ) && started
        }
        TournamentChildRoute::Bracket => {
            matches!(
                format,
                Format::SingleElimination | Format::DoubleElimination
            ) && started
        }
        TournamentChildRoute::Scheduling => {
            scheduling_eligible && status == TournamentStatus::InProgress
        }
        TournamentChildRoute::Manage => organizer && status != TournamentStatus::Finished,
        TournamentChildRoute::ManageMatches => organizer && status == TournamentStatus::InProgress,
    };
    if !auth_resolved
        && matches!(
            route,
            TournamentChildRoute::Scheduling
                | TournamentChildRoute::Manage
                | TournamentChildRoute::ManageMatches
        )
    {
        None
    } else {
        Some(available)
    }
}

fn tournament_child_suffix<'a>(path: &'a str, tournament_id: &TournamentId) -> Option<&'a str> {
    let root = format!("/tournament/{}", tournament_id.0);
    path.strip_prefix(&root).and_then(|suffix| {
        suffix
            .strip_prefix('/')
            .or_else(|| suffix.is_empty().then_some(""))
    })
}

fn current_tournament_path_matches(pathname: &str, tournament_id: &TournamentId) -> bool {
    tournament_path_matches(pathname, tournament_id)
}

fn child_route_link_class(current: &str, target: &str) -> &'static str {
    if current == target {
        "ui-button ui-button-primary ui-button-sm no-link-style"
    } else {
        "ui-button ui-button-ghost ui-button-sm no-link-style"
    }
}

fn selection_exists(
    lifecycle: &TournamentLifecycleDetails,
    memberships: &TournamentMemberships,
    selection: TournamentSelection,
) -> bool {
    if lifecycle.status == TournamentStatus::NotStarted {
        return false;
    }
    memberships.players.contains_key(&selection.player())
}

fn player_selection_from_query(
    memberships: &TournamentMemberships,
    requested_player: &str,
) -> Option<TournamentSelection> {
    let requested_player = requested_player.trim();
    let player = Uuid::parse_str(requested_player)
        .ok()
        .filter(|player| memberships.players.contains_key(player))
        .or_else(|| {
            memberships
                .players
                .iter()
                .find(|(_, player)| player.username.eq_ignore_ascii_case(requested_player))
                .map(|(player, _)| *player)
        })?;
    Some(TournamentSelection::Player(player))
}

fn player_query_from_selection(
    memberships: &TournamentMemberships,
    selection: TournamentSelection,
) -> Option<String> {
    memberships
        .players
        .get(&selection.player())
        .map(|player| player.username.clone())
}

#[component]
fn TournamentSurface(
    tournament: TournamentState,
    schedules: TournamentScheduleState,
    schedules_ready: Signal<bool>,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let identity = auth.identity;
    let admin = auth.admin;
    let user_id = Signal::derive(move || identity.get().and_then(AuthIdentity::user_id));
    let user_joined = Signal::derive(move || {
        user_id.get().is_some_and(|id| {
            tournament
                .common
                .memberships()
                .get()
                .players
                .contains_key(&id)
        })
    });
    let format = tournament.format.format();
    let realtime_format = match tournament.format {
        TournamentFormatStore::Arena(_) => false,
        TournamentFormatStore::RoundRobin(state) => {
            state.configuration().get_untracked().clock.mode() == TimeMode::RealTime
        }
        TournamentFormatStore::Swiss(state) => {
            state.configuration().get_untracked().clock.mode() == TimeMode::RealTime
        }
        TournamentFormatStore::Elimination(state) => {
            state.configuration().get_untracked().time_mode() == Some(TimeMode::RealTime)
        }
    };
    let scheduling_available = Signal::derive(move || {
        let Some(user_id) = user_id.get() else {
            return false;
        };
        let lifecycle = tournament.common.lifecycle().get();
        let memberships = tournament.common.memberships().get();
        if lifecycle.status != TournamentStatus::InProgress
            || !memberships.players.contains_key(&user_id)
            || memberships.withdrawn.contains(&user_id)
        {
            return false;
        }
        realtime_format
            && match tournament.format {
                TournamentFormatStore::Elimination(state) => state
                    .player_results()
                    .get()
                    .iter()
                    .any(|result| result.player == user_id && result.in_contention),
                _ => true,
            }
    });
    let organizer = Signal::derive(move || {
        let Some(id) = identity.get().and_then(AuthIdentity::user_id) else {
            return false;
        };
        admin.get().unwrap_or(false)
            || tournament
                .common
                .memberships()
                .get()
                .organizers
                .iter()
                .any(|organizer| organizer.uid == id)
    });
    let now = use_ticking_now();
    let pre_start_state = Signal::derive(move || {
        let lifecycle = tournament.common.lifecycle().get();
        pre_start_boundary(
            lifecycle.status == TournamentStatus::NotStarted,
            format,
            lifecycle.starts_at,
            now.get(),
        )
    });
    let pre_start_changes_open = Signal::derive(move || {
        pre_start_state.get() == PreStartBoundary::Open
            && !tournament
                .common
                .lifecycle()
                .get()
                .start_setup
                .is_some_and(|setup| setup.active_at(now.get()))
    });
    let scheduled_start_pending = Signal::derive(move || {
        pre_start_state.get() == PreStartBoundary::WaitingForScheduledWorker
    });
    let auth_resolved = Signal::derive(move || identity.get().is_some());
    let page_context = TournamentPageContext {
        tournament_id: StoredValue::new(tournament.tournament_id()),
        tournament,
        schedules,
        schedules_ready,
        user_id,
        user_joined,
        scheduling_available,
        organizer,
        auth_resolved,
        pre_start_changes_open,
        scheduled_start_pending,
        format,
    };
    provide_context(page_context);

    view! {
        <Show when=move || format != Format::Arena>
            <TournamentNavigation />
        </Show>
        <main class="min-w-0">
            <Outlet />
        </main>
    }
}

#[component]
fn TournamentNavigation() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let tournament_id = StoredValue::new(context.tournament.tournament_id());
    let pathname = use_location().pathname;
    let current_suffix = Memo::new(move |_| {
        let tournament_id = tournament_id.get_value();
        tournament_child_suffix(&pathname.get(), &tournament_id)
            .unwrap_or_default()
            .to_string()
    });
    let route_tournament = context.tournament;
    let route_visible = |route| {
        let availability = route_availability_memo(context, route);
        move || {
            let Some(pathname) = pathname.try_get() else {
                return false;
            };
            let Some(tournament_id) = tournament_id.try_get_value() else {
                return false;
            };
            current_tournament_path_matches(&pathname, &tournament_id)
                && route_tournament
                    .common
                    .lifecycle()
                    .try_get()
                    .is_some_and(|lifecycle| lifecycle.tournament_id == tournament_id)
                && availability.try_get() == Some(Some(true))
        }
    };
    let link_class = |target: &'static str, prefix: bool| {
        move || {
            let Some(pathname) = pathname.try_get() else {
                return child_route_link_class("", target);
            };
            let Some(tournament_id) = tournament_id.try_get_value() else {
                return child_route_link_class("", target);
            };
            if !current_tournament_path_matches(&pathname, &tournament_id) {
                return child_route_link_class("", target);
            }
            let Some(suffix) = current_suffix.try_get() else {
                return child_route_link_class("", target);
            };
            if prefix && suffix.starts_with(target) {
                child_route_link_class(target, target)
            } else {
                child_route_link_class(&suffix, target)
            }
        }
    };
    let show_results = route_visible(TournamentChildRoute::Results);
    let show_pairings = route_visible(TournamentChildRoute::Pairings);
    let show_standings = route_visible(TournamentChildRoute::Standings);
    let show_bracket = route_visible(TournamentChildRoute::Bracket);
    let show_scheduling = route_visible(TournamentChildRoute::Scheduling);
    let show_manage = route_visible(TournamentChildRoute::Manage);
    let overview_class = link_class("", false);
    let results_class = link_class("results", false);
    let pairings_class = link_class("pairings", false);
    let standings_class = link_class("standings", false);
    let bracket_class = link_class("bracket", false);
    let scheduling_class = link_class("schedule", true);
    let manage_class = link_class("manage", true);
    view! {
        <nav class="flex sticky top-10 z-40 flex-wrap gap-1 p-1.5 min-w-0 rounded-lg border shadow-sm border-black/10 ui-top-bar-surface dark:border-white/10">
            // TODO: i18n once copy is approved.
            <A href="" exact=true attr:class=overview_class>
                "Overview"
            </A>
            <Show when=show_results>
                // TODO: i18n once copy is approved.
                <A href="results" attr:class=results_class>
                    "Results"
                </A>
            </Show>
            <Show when=show_pairings>
                // TODO: i18n once copy is approved.
                <A href="pairings" attr:class=pairings_class>
                    "Pairings"
                </A>
            </Show>
            <Show when=show_standings>
                // TODO: i18n once copy is approved.
                <A href="standings" attr:class=standings_class>
                    "Standings"
                </A>
            </Show>
            <Show when=show_bracket>
                // TODO: i18n once copy is approved.
                <A href="bracket" attr:class=bracket_class>
                    "Bracket"
                </A>
            </Show>
            <Show when=show_scheduling>
                // TODO: i18n once copy is approved.
                <A href="schedule" attr:class=scheduling_class>
                    "Scheduling"
                </A>
            </Show>
            <Show when=show_manage>
                // TODO: i18n once copy is approved.
                <A href="manage" attr:class=manage_class>
                    "Manage"
                </A>
            </Show>
        </nav>
    }
}

#[component]
fn TournamentOverviewHeader() -> impl IntoView {
    let i18n = use_i18n();
    let context = expect_context::<TournamentPageContext>();
    let now = use_ticking_now();
    let arena_duration_seconds = match context.tournament.format {
        TournamentFormatStore::Arena(state) => {
            Some(state.configuration().get_untracked().duration_seconds.get())
        }
        _ => None,
    };
    let next_action = Signal::derive(move || {
        if !context.scheduling_available.get() || !context.schedules_ready.get() {
            return None;
        }
        let user_id = context.user_id.get()?;
        let count = participant_schedule_opponents_needing_time(
            context.tournament,
            context.schedules,
            user_id,
            now.get(),
        );
        // TODO: i18n once copy is approved.
        let label = if count > 0 {
            format!("Schedule games · {count} opponents")
        } else {
            "View schedule".to_string()
        };
        Some((
            label,
            format!(
                "/tournament/{}/schedule",
                context.tournament.tournament_id().0
            ),
        ))
    });
    let countdown = Signal::derive(move || {
        tournament_arena_countdown(
            &context.tournament.common.lifecycle().get(),
            arena_duration_seconds,
            now.get(),
        )
    });
    let show_description = Signal::derive(move || {
        context
            .tournament
            .common
            .lifecycle()
            .get()
            .description
            .is_some()
            || context.organizer.get()
    });

    view! {
        <header class="flex flex-wrap gap-3 justify-between items-center ui-panel-header">
            <h1 class="text-xl font-bold text-gray-900 dark:text-gray-100 wrap-break-word">
                {move || context.tournament.common.lifecycle().get().name}
            </h1>
            <div class="flex flex-wrap gap-3 items-center ml-auto">
                <Show when=move || countdown.get().is_some()>
                    <time
                        class="text-lg font-semibold tabular-nums text-gray-700 dark:text-gray-200"
                        aria-label=move || t_string!(i18n, tournaments.arena.time_left).to_string()
                    >
                        {move || countdown.get().unwrap_or_default()}
                    </time>
                </Show>
                <TournamentHeaderActions />
            </div>
        </header>
        <div class=move || {
            if context.tournament.common.lifecycle().get().status == TournamentStatus::Finished {
                "p-3 border-b border-black/10 dark:border-white/10 tournament-three:hidden"
            } else {
                "p-3 space-y-2 border-b sm:px-4 border-black/10 dark:border-white/10"
            }
        }>
            <div class="tournament-three:hidden">
                <TournamentEntryFacts tournament=context.tournament />
            </div>
            <TournamentParticipantAction
                common=context.tournament.common
                format=context.tournament.format
                next_action
            />
        </div>
        <Show when=show_description>
            <details class="border-b border-black/10 group dark:border-white/10">
                <summary class="py-3 px-4 text-sm list-none text-gray-700 transition-colors cursor-pointer dark:text-gray-200 [&::-webkit-details-marker]:hidden dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70">
                    <Show
                        when=move || {
                            context.tournament.common.lifecycle().get().description.is_some()
                        }
                        fallback=|| {
                            view! {
                                // TODO: i18n once copy is approved.
                                <span class="font-semibold text-pillbug-teal">
                                    "Add description"
                                </span>
                            }
                        }
                    >
                        <div class="grid gap-2 items-end grid-cols-[minmax(0,1fr)_auto] group-open:hidden">
                            <div
                                class="overflow-hidden max-w-none leading-6 wrap-break-word line-clamp-2 [&_p]:inline"
                                inner_html=move || {
                                    context
                                        .tournament
                                        .common
                                        .lifecycle()
                                        .get()
                                        .description
                                        .as_deref()
                                        .and_then(markdown_to_html)
                                        .unwrap_or_default()
                                }
                            />
                            // TODO: i18n once copy is approved.
                            <span class="inline-flex gap-1 items-center font-semibold whitespace-nowrap text-pillbug-teal">
                                "Read more"
                                <Icon icon=icondata_lu::LuChevronDown attr:class="size-4" />
                            </span>
                        </div>
                        // TODO: i18n once copy is approved.
                        <span class="hidden gap-1 justify-end items-center font-semibold text-pillbug-teal group-open:flex">
                            "Show less" <Icon icon=icondata_lu::LuChevronUp attr:class="size-4" />
                        </span>
                    </Show>
                </summary>
                <div class="pt-0 ui-panel-body">
                    <TournamentDescriptionEditor
                        tournament=context.tournament
                        editable=context.organizer
                    />
                </div>
            </details>
        </Show>
    }
}

#[component]
fn TournamentHeaderActions() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let tournament_id = StoredValue::new(context.tournament.tournament_id());
    let now = use_ticking_now();
    let active_setup = Memo::new(move |_| {
        context
            .tournament
            .common
            .lifecycle()
            .get()
            .start_setup
            .filter(|setup| setup.active_at(now.get()))
    });
    let pre_start = Signal::derive(move || {
        context.tournament.common.lifecycle().get().status == TournamentStatus::NotStarted
    });
    let manual_start = Signal::derive(move || {
        context.organizer.get() && context.format != Format::Arena && {
            let lifecycle = context.tournament.common.lifecycle().get();
            lifecycle.status == TournamentStatus::NotStarted && lifecycle.starts_at.is_none()
        }
    });
    let start_available = Signal::derive(move || {
        context.pre_start_changes_open.get() && {
            let lifecycle = context.tournament.common.lifecycle().get();
            let minimum = if matches!(context.format, Format::Swiss | Format::DoubleSwiss) {
                usize::try_from(lifecycle.min_seats.max(5)).unwrap_or(usize::MAX)
            } else {
                usize::try_from(lifecycle.min_seats).unwrap_or(usize::MAX)
            };
            context.tournament.common.memberships().get().players.len() >= minimum
        }
    });
    view! {
        <Show when=move || active_setup.get().is_some()>
            // TODO: i18n once copy is approved.
            <p class="text-sm text-amber-700 dark:text-amber-300">
                "Bracket review is in progress. The roster is locked until its owner finishes, cancels, or the 15-minute window ends."
            </p>
        </Show>
        <For
            each=move || {
                active_setup
                    .get()
                    .filter(|setup| Some(setup.owner_id) == context.user_id.get())
                    .into_iter()
            }
            key=|setup| setup.id
            let:setup
        >
            <EliminationStartSetup setup />
        </For>
        <Show when=context.organizer>
            <div class="flex flex-wrap gap-2 justify-end">
                <Show when=manual_start>
                    // TODO: i18n once copy is approved.
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-sm"
                        prop:disabled=move || !start_available.get()
                        on:click=move |_| {
                            if start_available.get_untracked() {
                                let mut expected_entrant_ids = context
                                    .tournament
                                    .common
                                    .memberships()
                                    .get_untracked()
                                    .players
                                    .into_keys()
                                    .collect::<Vec<_>>();
                                expected_entrant_ids.sort_unstable();
                                api.get()
                                    .tournament(
                                        TournamentAction::Start(
                                            tournament_id.get_value(),
                                            TournamentStartIntent {
                                                expected_entrant_ids,
                                            },
                                        ),
                                    );
                            }
                        }
                    >
                        "Start"
                    </button>
                </Show>
                <Show when=pre_start>
                    <button
                        type="button"
                        class="ui-button ui-button-danger ui-button-sm"
                        prop:disabled=move || !context.pre_start_changes_open.get()
                        on:click=move |_| {
                            if context.pre_start_changes_open.get_untracked()
                                && confirm_tournament_deletion("Delete this tournament?")
                            {
                                api.get()
                                    .tournament(
                                        TournamentAction::Delete(tournament_id.get_value()),
                                    );
                            }
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Delete"
                    </button>
                </Show>
            </div>
        </Show>
    }
}

#[component]
fn EliminationStartSetup(setup: TournamentStartSetup) -> impl IntoView {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    let context = expect_context::<TournamentPageContext>();
    let websocket = expect_context::<WebsocketContext>();
    let dialog = NodeRef::<Dialog>::new();
    let now = use_ticking_now();
    let deadline = setup.expires_at();
    let setup_id = setup.id;
    let tournament_id = context.tournament.tournament_id();
    let order = RwSignal::new(setup.seeded_players.clone());
    let position = RwSignal::new(None::<BracketPosition>);
    let defaults = StoredValue::new(setup.seeded_players.clone());
    let closed = Arc::new(AtomicBool::new(false));
    let cancel_closed = Arc::clone(&closed);
    let cancel_socket = websocket.clone();
    let cancel_tournament = tournament_id.clone();
    let cancel = Callback::new(move |()| {
        if !cancel_closed.swap(true, Ordering::Relaxed) && Utc::now() < deadline {
            cancel_socket.send(&ClientRequest::Tournament(TournamentAction::StartCancel(
                cancel_tournament.clone(),
                setup_id,
            )));
        }
    });
    let pathname = use_location().pathname;
    let opened_path = StoredValue::new(pathname.get_untracked());
    Effect::new(move |_| {
        if pathname.get() != opened_path.get_value() {
            cancel.run(());
            if let Some(dialog) = dialog.get() {
                dialog.close();
            }
        }
    });
    let _ = use_event_listener(use_window(), pagehide, move |_| cancel.run(()));
    let cleanup_closed = Arc::clone(&closed);
    let cleanup_socket = websocket.clone();
    let cleanup_tournament = tournament_id.clone();
    let lifecycle: ArcField<TournamentLifecycleDetails> =
        context.tournament.common.lifecycle().into();
    on_cleanup(move || {
        let still_active = lifecycle.try_get_untracked().is_some_and(|lifecycle| {
            lifecycle
                .start_setup
                .is_some_and(|setup| setup.id == setup_id)
        });
        if still_active && !cleanup_closed.swap(true, Ordering::Relaxed) && Utc::now() < deadline {
            cleanup_socket.send(&ClientRequest::Tournament(TournamentAction::StartCancel(
                cleanup_tournament,
                setup_id,
            )));
        }
    });
    Effect::new(move |_| {
        if let Some(dialog) = dialog.get() {
            let _ = dialog.show_modal();
        }
    });
    let configuration = match context.tournament.format {
        TournamentFormatStore::Elimination(state) => Some(state.configuration().get_untracked()),
        _ => None,
    };
    let memberships = Signal::derive(move || context.tournament.common.memberships().get());
    // TODO: i18n once copy is approved.
    view! {
        <Modal
            dialog_el=dialog
            aria_label="Review elimination bracket"
            on_close=cancel
            hide_close_button=true
        >
            <div
                class="flex flex-col w-[min(94vw,96rem)] h-[calc(90dvh-2px)]"
                on:keydown=move |event| {
                    if event.key() == "Escape" && position.get_untracked().is_some() {
                        event.prevent_default();
                        event.stop_propagation();
                        position.set(None);
                    }
                }
            >
                <header class="py-2 px-3 border-b border-gray-200 sm:px-4 dark:border-gray-700 shrink-0">
                    <div class="flex gap-3 justify-between items-center">
                        <h2 class="text-lg font-bold sm:text-xl">"Review bracket"</h2>
                        <div class="flex gap-3 items-center">
                            <span class="text-sm font-semibold tabular-nums" title="Time remaining">
                                {move || {
                                    let seconds = (deadline - now.get()).num_seconds().max(0);
                                    format!("{}:{:02}", seconds / 60, seconds % 60)
                                }}
                            </span>
                            <button
                                type="button"
                                class="ui-button ui-button-ghost ui-button-icon size-10"
                                aria-label="Close bracket review"
                                on:click=move |_| {
                                    cancel.run(());
                                    if let Some(dialog) = dialog.get() {
                                        dialog.close();
                                    }
                                }
                            >
                                "×"
                            </button>
                        </div>
                    </div>
                    <p class="text-xs text-gray-600 dark:text-gray-400">
                        "Select a position to assign a player."
                    </p>
                </header>
                <div
                    class="overflow-y-auto flex-1 py-3 px-3 min-h-0 sm:px-4"
                    data-testid="bracket-editor-scroll"
                >
                    {configuration
                        .map(|configuration| {
                            view! {
                                <details class="mb-3 text-sm">
                                    <summary class="font-semibold cursor-pointer">
                                        "Series and color rules"
                                    </summary>
                                    <div class="mt-2">
                                        <TournamentFormatConfigurationDetails configuration=FormatConfig::Elimination(
                                            configuration.clone(),
                                        ) />
                                    </div>
                                </details>
                                <BracketPlacementPreview
                                    configuration
                                    seeded_players=setup.seeded_players
                                    order
                                    position
                                    memberships
                                />
                            }
                        })}
                </div>
                <footer class="flex flex-wrap gap-2 items-center py-3 px-3 border-t border-gray-200 sm:px-4 dark:border-gray-700 shrink-0">
                    <button
                        type="button"
                        class="mr-auto min-h-10 ui-button ui-button-ghost ui-button-sm"
                        title="Reset to seeded bracket"
                        on:click=move |_| {
                            position.set(None);
                            order.set(defaults.get_value());
                        }
                    >
                        "Reset"
                    </button>
                    <button
                        type="button"
                        class="min-h-10 ui-button ui-button-secondary ui-button-sm"
                        on:click=move |_| {
                            cancel.run(());
                            if let Some(dialog) = dialog.get() {
                                dialog.close();
                            }
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        type="button"
                        class="min-h-10 ui-button ui-button-primary ui-button-sm"
                        prop:disabled=move || { now.get() >= deadline }
                        on:click=move |_| {
                            if now.get_untracked() < deadline {
                                websocket
                                    .send(
                                        &ClientRequest::Tournament(
                                            TournamentAction::StartConfirm(
                                                tournament_id.clone(),
                                                setup_id,
                                                order.get_untracked(),
                                            ),
                                        ),
                                    );
                            }
                        }
                    >
                        "Confirm and start"
                    </button>
                </footer>
            </div>
        </Modal>
    }
}

#[component]
fn TournamentChatUtility(visible: Signal<bool>) -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let tournament_id = StoredValue::new(context.tournament.tournament_id());
    view! {
        <Show when=move || {
            visible.get() && (context.organizer.get() || context.user_joined.get())
        }>
            <section class="flex overflow-hidden flex-col min-w-0 h-96 ui-panel tournament-three:h-auto tournament-three:flex-1 tournament-three:min-h-0">
                // TODO: i18n once copy is approved.
                <h2 class="font-bold shrink-0 ui-panel-header">"Tournament chat"</h2>
                <div class="flex overflow-hidden flex-col flex-1 min-h-0">
                    <ResolvedChatWindow
                        conversation=ConversationKey::Tournament(tournament_id.get_value())
                        compact=true
                    />
                </div>
            </section>
        </Show>
    }
}

fn route_availability(
    context: &TournamentPageContext,
    route: TournamentChildRoute,
) -> Option<bool> {
    child_route_availability(
        route,
        context.format,
        context.tournament.common.lifecycle().try_get()?.status,
        context.scheduling_available.try_get()?,
        context.organizer.try_get()?,
        context.auth_resolved.try_get()?,
    )
}

fn tournament_child_route_condition(route: TournamentChildRoute) -> Option<bool> {
    let context = use_context::<TournamentPageContext>()?;
    let pathname = use_location().pathname.try_get()?;
    let tournament_id = context.tournament_id.try_get_value()?;
    if !current_tournament_path_matches(&pathname, &tournament_id) {
        return None;
    }
    if !context
        .tournament
        .common
        .lifecycle()
        .try_get()
        .is_some_and(|lifecycle| lifecycle.tournament_id == tournament_id)
    {
        return None;
    }
    route_availability(&context, route)
}

fn tournament_overview_path() -> String {
    let pathname = use_location()
        .pathname
        .try_get()
        .unwrap_or_else(|| String::from("/"));
    let Some(context) = use_context::<TournamentPageContext>() else {
        return pathname;
    };
    let Some(tournament_id) = context.tournament_id.try_get_value() else {
        return pathname;
    };
    if current_tournament_path_matches(&pathname, &tournament_id) {
        format!("/tournament/{}", tournament_id.0)
    } else {
        pathname
    }
}

fn route_availability_memo(
    context: TournamentPageContext,
    route: TournamentChildRoute,
) -> Memo<Option<bool>> {
    Memo::new(move |_| route_availability(&context, route))
}

#[component]
fn TournamentAccessPending() -> impl IntoView {
    view! {
        <Panel>
            // TODO: i18n once copy is approved.
            <p class="text-sm text-gray-600 dark:text-gray-300">"Loading tournament access…"</p>
        </Panel>
    }
}

#[component(transparent)]
pub fn TournamentRoutes() -> impl MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("/tournament/:nanoid") view=Tournament>
            <Route path=path!("") view=OverviewRoute />
            <ProtectedRoute
                condition=|| tournament_child_route_condition(TournamentChildRoute::Results)
                path=path!("results")
                redirect_path=tournament_overview_path
                fallback=TournamentAccessPending
                view=ResultsRoute
            />
            <ProtectedRoute
                condition=|| tournament_child_route_condition(TournamentChildRoute::Pairings)
                path=path!("pairings")
                redirect_path=tournament_overview_path
                fallback=TournamentAccessPending
                view=PairingsRoute
            />
            <ProtectedRoute
                condition=|| tournament_child_route_condition(TournamentChildRoute::Standings)
                path=path!("standings")
                redirect_path=tournament_overview_path
                fallback=TournamentAccessPending
                view=TournamentStandingsRoute
            />
            <ProtectedRoute
                condition=|| tournament_child_route_condition(TournamentChildRoute::Bracket)
                path=path!("bracket")
                redirect_path=tournament_overview_path
                fallback=TournamentAccessPending
                view=BracketRoute
            />
            <ProtectedParentRoute
                condition=|| tournament_child_route_condition(TournamentChildRoute::Scheduling)
                path=path!("schedule")
                redirect_path=tournament_overview_path
                fallback=TournamentAccessPending
                view=TournamentSchedulingRoute
            >
                <Route path=path!("") view=ParticipantScheduleIndex />
                <Route path=path!(":username") view=ParticipantScheduleOpponent />
            </ProtectedParentRoute>
            <ProtectedParentRoute
                condition=|| tournament_child_route_condition(TournamentChildRoute::Manage)
                path=path!("manage")
                redirect_path=tournament_overview_path
                fallback=TournamentAccessPending
                view=TournamentManageRoute
            >
                <Route path=path!("") view=TournamentManageIndex />
                <ProtectedRoute
                    condition=|| {
                        tournament_child_route_condition(TournamentChildRoute::ManageMatches)
                    }
                    path=path!("matches/:slot_id")
                    redirect_path=tournament_overview_path
                    fallback=TournamentAccessPending
                    view=TournamentManageMatchDetailRoute
                />
            </ProtectedParentRoute>
            <Route path=path!("*rest") view=TournamentInvalidRoute />
        </ParentRoute>
    }
    .into_inner()
    .into_any_nested_route()
}

#[component]
pub fn OverviewRoute() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let tournament = context.tournament;
    let selection = RwSignal::new(None::<TournamentSelection>);
    let (requested_player, set_requested_player) = query_signal_with_options::<String>(
        "player",
        NavigateOptions {
            resolve: true,
            replace: true,
            scroll: false,
            ..Default::default()
        },
    );
    let rail = NodeRef::<Aside>::new();
    let rail_size = use_element_size(rail);
    let chat_visible = RwSignal::new(true);
    Effect::new(move |_| {
        let requested_selection = requested_player.get().and_then(|requested_player| {
            player_selection_from_query(&tournament.common.memberships().get(), &requested_player)
        });
        if selection.get_untracked() != requested_selection {
            selection.set(requested_selection);
        }
    });
    Effect::new(move |_| {
        let selected_player = selection.get().and_then(|selection| {
            player_query_from_selection(&tournament.common.memberships().get(), selection)
        });
        if requested_player.get_untracked() != selected_player {
            set_requested_player.set(selected_player);
        }
    });
    Effect::new(move |_| {
        let Some(selected) = selection.get() else {
            return;
        };
        if !selection_exists(
            &tournament.common.lifecycle().get(),
            &tournament.common.memberships().get(),
            selected,
        ) {
            selection.set(None);
        }
    });
    view! {
        <div class="grid gap-3 items-start min-w-0 tournament-two:grid-cols-[minmax(0,1fr)_minmax(16rem,20rem)] tournament-three:min-h-[calc(100vh-7rem)] tournament-three:items-stretch tournament-three:grid-cols-[clamp(18rem,16vw,24rem)_minmax(0,1fr)_clamp(18rem,16vw,24rem)]">
            <div class="min-w-0 tournament-two:col-start-1 tournament-two:row-start-1 tournament-three:col-start-2">
                <TournamentOverview tournament selection />
            </div>
            <aside class="contents min-w-0 tournament-two:block tournament-two:col-start-2 tournament-two:row-start-1 tournament-two:row-span-2 tournament-three:col-start-3 tournament-three:row-span-1">
                <TournamentOverviewInspector tournament selection>
                    <TournamentOverviewRail tournament selection />
                </TournamentOverviewInspector>
            </aside>
            <aside
                node_ref=rail
                class="flex flex-col gap-3 min-w-0 tournament-two:col-start-1 tournament-two:row-start-2 tournament-three:col-start-1 tournament-three:row-start-1 tournament-three:h-full tournament-three:max-h-[calc(100vh-7rem)]"
            >
                <TournamentInformation
                    tournament
                    scheduled_start_pending=context.scheduled_start_pending
                    rail_height=rail_size.height
                    chat_visible
                />
                <TournamentOrganizers tournament user_is_organizer_or_admin=context.organizer />
                <TournamentChatUtility visible=chat_visible.into() />
            </aside>
        </div>
    }
}

#[component]
pub fn ResultsRoute() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let TournamentFormatStore::RoundRobin(round_robin) = context.tournament.format else {
        unreachable!("results route is only available for Round Robin tournaments")
    };
    view! {
        <RoundRobinCrosstable
            common=context.tournament.common
            round_robin
            user_id=context.user_id
        />
    }
}

#[component]
pub fn PairingsRoute() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let TournamentFormatStore::Swiss(swiss) = context.tournament.format else {
        unreachable!("pairings route is only available for Swiss tournaments")
    };
    view! {
        <div class="relative left-1/2 -translate-x-1/2 w-[calc(100vw-0.75rem)] sm:w-[98vw]">
            <SwissRoundBrowser common=context.tournament.common swiss />
        </div>
    }
}

#[component]
pub fn TournamentStandingsRoute() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    view! { <TournamentDetailedStandings tournament=context.tournament /> }
}

#[component]
pub fn BracketRoute() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let TournamentFormatStore::Elimination(elimination) = context.tournament.format else {
        unreachable!("bracket route is only available for Elimination tournaments")
    };
    let selection = RwSignal::new(None::<TournamentSelection>);
    view! {
        <div class="relative left-1/2 -translate-x-1/2 w-[calc(100vw-0.75rem)] sm:w-[98vw]">
            <Bracket common=context.tournament.common elimination selection />
        </div>
    }
}

#[component]
pub fn TournamentSchedulingRoute() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    view! {
        <ParticipantScheduleLayout
            tournament=context.tournament
            schedules=context.schedules
            user_id=context.user_id
        />
    }
}

#[component]
pub fn TournamentManageRoute() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let tournament_id = StoredValue::new(context.tournament.tournament_id());
    let pathname = use_location().pathname;
    let in_progress = Memo::new(move |_| {
        context.tournament.common.lifecycle().get().status == TournamentStatus::InProgress
    });
    view! {
        {move || {
            let pathname = pathname.try_get()?;
            let tournament_id = tournament_id.try_get_value()?;
            if !current_tournament_path_matches(&pathname, &tournament_id)
                || context
                    .tournament
                    .common
                    .lifecycle()
                    .try_get()
                    .is_none_or(|lifecycle| lifecycle.tournament_id != tournament_id)
            {
                return None;
            }
            Some(
                if in_progress.try_get()? {
                    view! {
                        <TournamentLiveManage
                            tournament=context.tournament
                            schedules=context.schedules
                            schedules_ready=context.schedules_ready
                            organizer=context.organizer
                        />
                    }
                        .into_any()
                } else {
                    view! { <Outlet /> }.into_any()
                },
            )
        }}
    }
}

#[component]
pub fn TournamentManageIndex() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let i18n = use_i18n();
    let tournament_id = StoredValue::new(context.tournament.tournament_id());
    let pathname = use_location().pathname;
    let in_progress = Memo::new(move |_| {
        context.tournament.common.lifecycle().get().status == TournamentStatus::InProgress
    });
    view! {
        {move || {
            let pathname = pathname.try_get()?;
            let tournament_id = tournament_id.try_get_value()?;
            if !current_tournament_path_matches(&pathname, &tournament_id)
                || context
                    .tournament
                    .common
                    .lifecycle()
                    .try_get()
                    .is_none_or(|lifecycle| lifecycle.tournament_id != tournament_id)
            {
                return None;
            }
            Some(
                if in_progress.try_get()? {
                    view! { <TournamentOrganizerIndex /> }.into_any()
                } else {
                    view! {
                        <Panel
                            title=move || context.tournament.common.lifecycle().get().name
                            body_class="space-y-3"
                        >
                            <Show when=context.scheduled_start_pending>
                                <p class="ui-notice">
                                    {t!(i18n, tournaments.detail.start_worker_pending)}
                                </p>
                            </Show>
                            <TournamentAdminControls
                                user_is_organizer_or_admin=context.organizer
                                tournament=context.tournament
                                editable=context.pre_start_changes_open
                            />
                        </Panel>
                    }
                        .into_any()
                },
            )
        }}
    }
}

#[component]
pub fn TournamentManageMatchDetailRoute() -> impl IntoView {
    view! { <TournamentOrganizerMatch /> }
}

#[component]
pub fn TournamentInvalidRoute() -> impl IntoView {
    let context = expect_context::<TournamentPageContext>();
    let root = format!("/tournament/{}", context.tournament.tournament_id().0);
    view! {
        <Redirect
            path=root
            options=NavigateOptions {
                replace: true,
                ..Default::default()
            }
        />
    }
}

#[component]
fn TournamentEntryFacts(tournament: TournamentState) -> impl IntoView {
    let i18n = use_i18n();
    let clock = match tournament.format {
        TournamentFormatStore::Arena(state) => Some(shared_types::Clock::Realtime(
            state.configuration().get_untracked().game_clock,
        )),
        TournamentFormatStore::RoundRobin(state) => {
            Some(state.configuration().get_untracked().clock)
        }
        TournamentFormatStore::Swiss(state) => Some(state.configuration().get_untracked().clock),
        TournamentFormatStore::Elimination(state) => state
            .configuration()
            .get_untracked()
            .default_plan
            .phases
            .first()
            .map(|phase| phase.clock),
    };
    view! {
        <div class="space-y-1 text-xs text-gray-600 dark:text-gray-300">
            <div class="flex flex-wrap gap-y-1 gap-x-2 items-center">
                {clock.map(|clock| view! { <TimeRow time_control=Some(clock) /> })}
                <span>{move || tournament_format_label(i18n, tournament.format.format())}</span>
            </div>
            {move || {
                let lifecycle = tournament.common.lifecycle().get();
                (lifecycle.status == TournamentStatus::NotStarted
                    || (tournament.format.format() == Format::Arena
                        && lifecycle.status == TournamentStatus::InProgress))
                    .then(|| {
                        view! {
                            // TODO: i18n once copy is approved.
                            <p>
                                {lifecycle
                                    .starts_at
                                    .map(|date| {
                                        format!("Starts {}", format_tournament_datetime(date))
                                    })
                                    .unwrap_or_else(|| {
                                        "Starts when the organizer begins the tournament"
                                            .to_string()
                                    })}
                            </p>
                            <p>
                                {entry_requirement(
                                    &lifecycle,
                                    tournament.common.bot_admission().get(),
                                )}
                            </p>
                        }
                    })
            }}
        </div>
    }
}

fn entry_requirement(
    tournament: &TournamentLifecycleDetails,
    bot_admission: BotAdmission,
) -> String {
    // TODO: i18n once copy is approved.
    let rating = match (tournament.band_lower, tournament.band_upper) {
        (None, None) => None,
        (Some(lower), None) => Some(format!("{lower}+")),
        (None, Some(upper)) => Some(format!("≤ {upper}")),
        (Some(lower), Some(upper)) => Some(format!("{lower}–{upper}")),
    };
    let entry = match (tournament.invite_only, rating) {
        (false, None) => String::from("Open entry"),
        (true, None) => String::from("Invitation required"),
        (false, Some(rating)) => format!("Entry rating {rating}"),
        (true, Some(rating)) => format!("Invitation required · rating {rating}"),
    };
    // TODO: i18n once copy is approved.
    match bot_admission {
        BotAdmission::HumansOnly => format!("{entry} · human accounts only"),
        BotAdmission::BotsOnly => format!("{entry} · bot accounts only"),
        BotAdmission::HumansAndBots => entry,
    }
}

fn primary_tournament_date(tournament: &TournamentLifecycleDetails) -> Option<DateTime<Utc>> {
    match tournament.status {
        TournamentStatus::NotStarted => tournament.starts_at.or(Some(tournament.created_at)),
        TournamentStatus::InProgress => tournament.started_at.or(tournament.starts_at),
        TournamentStatus::Finished => tournament.finished_at.or(tournament.started_at),
    }
}

fn tournament_arena_countdown(
    tournament: &TournamentLifecycleDetails,
    arena_duration_seconds: Option<u32>,
    now: DateTime<Utc>,
) -> Option<String> {
    if tournament.status == TournamentStatus::Finished {
        return None;
    }
    let starts_at = tournament.starts_at?;
    let ends_at = starts_at + Duration::seconds(i64::from(arena_duration_seconds?));
    if now < starts_at || now >= ends_at {
        return None;
    }
    let total = ends_at.signed_duration_since(now).num_seconds().max(0);
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    Some(if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    })
}

#[component]
fn TournamentInformation(
    tournament: TournamentState,
    scheduled_start_pending: Signal<bool>,
    rail_height: Signal<f64>,
    chat_visible: RwSignal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let format = tournament.format.format();
    let (format_configuration, clock, arena_duration) = match tournament.format {
        TournamentFormatStore::Arena(state) => {
            let configuration = state.configuration().get_untracked();
            (
                FormatConfig::Arena(configuration.clone()),
                Some(shared_types::Clock::Realtime(configuration.game_clock)),
                Some(configuration.duration_seconds.get()),
            )
        }
        TournamentFormatStore::RoundRobin(state) => {
            let configuration = state.configuration().get_untracked();
            (
                FormatConfig::RoundRobin(configuration.clone()),
                Some(configuration.clock),
                None,
            )
        }
        TournamentFormatStore::Swiss(state) => {
            let configuration = state.configuration().get_untracked();
            (
                FormatConfig::Swiss(configuration.clone()),
                Some(configuration.clock),
                None,
            )
        }
        TournamentFormatStore::Elimination(state) => {
            let configuration = state.configuration().get_untracked();
            let clock = configuration
                .default_plan
                .phases
                .first()
                .map(|phase| phase.clock);
            (
                FormatConfig::Elimination(configuration.clone()),
                clock,
                None,
            )
        }
    };
    let speed = clock.map(GameSpeed::from);
    let format_configuration = StoredValue::new(format_configuration);
    let show_scoring = matches!(
        format,
        Format::RoundRobin | Format::Swiss | Format::DoubleSwiss
    );
    let show_entry_context = Signal::derive(move || {
        let lifecycle = tournament.common.lifecycle().get();
        lifecycle.status == TournamentStatus::NotStarted
            || (format == Format::Arena && lifecycle.status == TournamentStatus::InProgress)
    });
    let information_summary = NodeRef::<Summary>::new();
    let information_body = NodeRef::<Div>::new();
    let information_panel = NodeRef::<Details>::new();
    let information_open = RwSignal::new(false);
    let summary_size = use_element_size(information_summary);
    let body_size = use_element_size(information_body);
    let utility_column_layout = use_media_query("(min-width: 1260px)");
    Effect::new(move |_| {
        let rail_height = rail_height.get();
        let summary_height = summary_size.height.get();
        let remaining_height = rail_height - summary_height - body_size.height.get() - 12.0;
        chat_visible.set(
            !utility_column_layout.get()
                || rail_height <= 0.0
                || summary_height <= 0.0
                || !information_open.get()
                || remaining_height >= 192.0,
        );
    });

    view! {
        <details
            node_ref=information_panel
            class="overflow-y-auto min-w-0 ui-panel group tournament-three:min-h-0"
            on:toggle=move |_| {
                if let Some(panel) = information_panel.get_untracked() {
                    information_open.set(panel.open());
                }
            }
        >
            <summary
                node_ref=information_summary
                class="sticky top-0 z-10 p-4 list-none transition-colors cursor-pointer focus-visible:ring-2 focus-visible:ring-inset focus-visible:outline-none bg-even-light/95 [&::-webkit-details-marker]:hidden dark:bg-surface-panel dark:hover:bg-pillbug-teal/15 hover:bg-blue-light/70 focus-visible:ring-pillbug-teal"
            >
                <div class="flex gap-2 items-center">
                    <div class="flex-1 space-y-1 min-w-0">
                        <div class="flex flex-wrap gap-y-1 gap-x-2 items-center font-semibold">
                            {clock.map(|clock| view! { <TimeRow time_control=Some(clock) /> })}
                            {speed
                                .map(|speed| {
                                    view! {
                                        <span class="text-gray-600 dark:text-gray-300">
                                            {speed.to_string()}
                                        </span>
                                    }
                                })} <span>{move || tournament_format_label(i18n, format)}</span>
                            {arena_duration
                                .map(|seconds| {
                                    view! {
                                        // TODO: i18n once copy is approved.
                                        <span>{format!("{} minutes", seconds / 60)}</span>
                                    }
                                })}
                        </div>
                        <p class="text-sm text-gray-600 dark:text-gray-300">
                            {move || {
                                primary_tournament_date(&tournament.common.lifecycle().get())
                                    .map(format_tournament_datetime)
                                    .unwrap_or_default()
                            }}
                        </p>
                        <Show when=move || {
                            tournament.common.lifecycle().get().status
                                == TournamentStatus::NotStarted
                        }>
                            <p class="text-sm font-semibold text-grasshopper-green">
                                {move || entry_requirement(
                                    &tournament.common.lifecycle().get(),
                                    tournament.common.bot_admission().get(),
                                )}
                            </p>
                        </Show>
                    </div>
                    <span class="flex justify-center items-center rounded-full transition-colors size-11 shrink-0 bg-pillbug-teal/10 text-pillbug-teal dark:bg-pillbug-teal/15">
                        <Icon
                            icon=icondata_lu::LuChevronDown
                            attr:class="size-5 transition-transform group-open:rotate-180"
                        />
                    </span>
                </div>
            </summary>
            <div node_ref=information_body class="pt-0 space-y-4 ui-panel-body">
                <div class="flex flex-wrap gap-2 items-center">
                    // TODO: i18n once copy is approved.
                    <span class="text-sm text-gray-500 dark:text-gray-400">"Organized by"</span>
                    <For
                        each=move || tournament.common.memberships().get().organizers
                        key=|organizer| organizer.uid
                        let:organizer
                    >
                        <UserIdentity
                            user=organizer
                            class="h-7"
                            link_class="truncate max-w-[12rem]"
                        />
                    </For>
                </div>
                <Show when=show_entry_context>
                    <div class="space-y-1 text-sm text-gray-700 dark:text-gray-300">
                        <p>
                            {move || entry_requirement(
                                &tournament.common.lifecycle().get(),
                                tournament.common.bot_admission().get(),
                            )}
                        </p>
                        <Show when=move || {
                            format != Format::Arena
                                && tournament.common.lifecycle().get().status
                                    == TournamentStatus::NotStarted
                        }>
                            // TODO: i18n once copy is approved.
                            <p>
                                {move || {
                                    format!(
                                        "Minimum {} entrants",
                                        tournament.common.lifecycle().get().min_seats,
                                    )
                                }}
                            </p>
                        </Show>
                        <Show when=scheduled_start_pending>
                            // TODO: i18n once copy is approved.
                            <p>"Starting… signup and configuration are briefly closed."</p>
                        </Show>
                    </div>
                </Show>
                <dl class="grid gap-2 text-sm sm:grid-cols-2">
                    <Show when=move || {
                        tournament.common.lifecycle().get().status == TournamentStatus::NotStarted
                    }>
                        <div>
                            // TODO: i18n once copy is approved.
                            <dt class="font-semibold">"Created"</dt>
                            <dd class="text-gray-600 dark:text-gray-300">
                                {move || {
                                    format_tournament_datetime(
                                        tournament.common.lifecycle().get().created_at,
                                    )
                                }}
                            </dd>
                        </div>
                    </Show>
                    <Show when=move || {
                        let lifecycle = tournament.common.lifecycle().get();
                        lifecycle.status == TournamentStatus::NotStarted
                            && lifecycle.starts_at.is_some()
                    }>
                        <div>
                            // TODO: i18n once copy is approved.
                            <dt class="font-semibold">"Scheduled"</dt>
                            <dd class="text-gray-600 dark:text-gray-300">
                                {move || {
                                    tournament
                                        .common
                                        .lifecycle()
                                        .get()
                                        .starts_at
                                        .map(format_tournament_datetime)
                                        .unwrap_or_default()
                                }}
                            </dd>
                        </div>
                    </Show>
                    <Show when=move || {
                        let lifecycle = tournament.common.lifecycle().get();
                        lifecycle.status == TournamentStatus::InProgress
                            && lifecycle.started_at.is_some()
                    }>
                        <div>
                            // TODO: i18n once copy is approved.
                            <dt class="font-semibold">"Started"</dt>
                            <dd class="text-gray-600 dark:text-gray-300">
                                {move || {
                                    tournament
                                        .common
                                        .lifecycle()
                                        .get()
                                        .started_at
                                        .map(format_tournament_datetime)
                                        .unwrap_or_default()
                                }}
                            </dd>
                        </div>
                    </Show>
                    <Show when=move || {
                        let lifecycle = tournament.common.lifecycle().get();
                        lifecycle.status == TournamentStatus::Finished
                            && lifecycle.finished_at.is_some()
                    }>
                        <div>
                            // TODO: i18n once copy is approved.
                            <dt class="font-semibold">"Finished"</dt>
                            <dd class="text-gray-600 dark:text-gray-300">
                                {move || {
                                    tournament
                                        .common
                                        .lifecycle()
                                        .get()
                                        .finished_at
                                        .map(format_tournament_datetime)
                                        .unwrap_or_default()
                                }}
                            </dd>
                        </div>
                    </Show>
                </dl>
                <Show when=move || format != Format::Arena>
                    <details class="ui-setting-group">
                        // TODO: i18n once copy is approved.
                        <summary class="font-semibold cursor-pointer">"Format details"</summary>
                        <div class="mt-3">
                            <TournamentFormatConfigurationDetails configuration=format_configuration
                                .get_value() />
                        </div>
                    </details>
                </Show>
                <Show when=move || show_scoring>
                    <details class="ui-setting-group">
                        // TODO: i18n once copy is approved.
                        <summary class="font-semibold cursor-pointer">
                            "Scoring and tiebreaks"
                        </summary>
                        <StandingsCriteriaRules configuration=format_configuration.get_value() />
                    </details>
                </Show>
                <details class="ui-setting-group">
                    // TODO: i18n once copy is approved.
                    <summary class="font-semibold cursor-pointer">"About this format"</summary>
                    <div class="pt-3">
                        <TournamentFormatFaq format=Signal::derive(move || format) />
                    </div>
                </details>
            </div>
        </details>
    }
}

#[component]
fn TournamentOverview(
    tournament: TournamentState,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    view! {
        <TournamentOverviewStandings tournament selection>
            <TournamentOverviewHeader />
        </TournamentOverviewStandings>
    }
}

#[component]
fn TournamentLiveManage(
    tournament: TournamentState,
    schedules: TournamentScheduleState,
    schedules_ready: Signal<bool>,
    organizer: Signal<bool>,
) -> impl IntoView {
    let tournament_actions_available =
        !matches!(tournament.format, TournamentFormatStore::Arena(_));
    let closeout = Signal::derive(move || match tournament.format {
        TournamentFormatStore::RoundRobin(state) => Some(state.closeout_eligible_slots().get()),
        TournamentFormatStore::Swiss(state) => Some(state.closeout_eligible_slots().get()),
        TournamentFormatStore::Arena(_) | TournamentFormatStore::Elimination(_) => None,
    });
    view! {
        <TournamentOrganizerLayout tournament schedules schedules_ready>
            <Show when=move || tournament_actions_available>
                <details class="relative w-full tournament-two:w-auto">
                    // TODO: i18n once copy is approved.
                    <summary class="list-none ui-button ui-button-secondary ui-button-sm [&::-webkit-details-marker]:hidden">
                        "Tournament actions"
                    </summary>
                    <div class="absolute right-0 z-30 p-3 mt-2 space-y-3 w-[min(28rem,calc(100vw-1.5rem))] ui-panel">
                        // TODO: i18n once copy is approved.
                        <h3 class="font-bold">"Participant withdrawal"</h3>
                        <TournamentWithdrawal
                            common=tournament.common
                            format=tournament.format
                            organizer
                        />
                        <Show when=move || closeout.get().is_some_and(|count| count > 0)>
                            <div class="pt-3 border-t border-black/10 dark:border-white/10">
                                <TournamentCloseout tournament organizer />
                            </div>
                        </Show>
                    </div>
                </details>
            </Show>
        </TournamentOrganizerLayout>
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DescriptionEditorState {
    current: Option<String>,
    draft: String,
    editing: bool,
    previewing: bool,
}

impl DescriptionEditorState {
    fn new(current: Option<String>) -> Self {
        Self {
            draft: current.clone().unwrap_or_default(),
            current,
            editing: false,
            previewing: false,
        }
    }

    fn sync_current(&mut self, current: Option<String>) {
        self.current = current;
        if !self.editing {
            self.draft = self.current.clone().unwrap_or_default();
        }
    }

    fn begin_edit(&mut self) {
        self.draft = self.current.clone().unwrap_or_default();
        self.editing = true;
        self.previewing = false;
    }

    fn cancel(&mut self) {
        self.draft = self.current.clone().unwrap_or_default();
        self.editing = false;
        self.previewing = false;
    }

    fn complete(&mut self, description: Option<String>) {
        self.current = description;
        self.cancel();
    }
}

#[component]
fn TournamentDescriptionEditor(
    tournament: TournamentState,
    editable: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let state = RwSignal::new(DescriptionEditorState::new(
        tournament.common.lifecycle().get_untracked().description,
    ));
    let update = ServerAction::<UpdateDescription>::new();
    let pending = update.pending();

    Effect::new(move |_| {
        let current = tournament.common.lifecycle().get().description;
        state.update(|state| state.sync_current(current));
    });
    Effect::new(move |_| {
        if !editable.get() {
            update.clear();
            state.update(DescriptionEditorState::cancel);
        }
    });
    Effect::new(move |_| {
        if let Some(Ok(description)) = update.value().get() {
            state.update(|state| state.complete(description.clone()));
            tournament.apply_patch(TournamentPatch::DescriptionChanged(description));
        }
    });

    view! {
        <div class="space-y-3 ui-setting-group">
            <Show
                when=move || state.with(|state| state.editing)
                fallback=move || {
                    view! {
                        <div
                            class="w-full max-w-none wrap-break-word prose dark:prose-invert"
                            inner_html=move || {
                                state
                                    .with(|state| {
                                        state
                                            .current
                                            .as_deref()
                                            .map(markdown_to_html)
                                            .unwrap_or_default()
                                    })
                            }
                        />
                        <Show when=editable>
                            <div class="flex flex-wrap gap-2">
                                <button
                                    type="button"
                                    class="ui-button ui-button-secondary ui-button-sm"
                                    on:click=move |_| {
                                        update.clear();
                                        state.update(DescriptionEditorState::begin_edit);
                                    }
                                >
                                    {t!(i18n, tournaments.detail.edit_description)}
                                </button>
                                <Show when=move || state.with(|state| state.current.is_some())>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-danger ui-button-sm"
                                        prop:disabled=move || pending.get()
                                        on:click=move |_| {
                                            update
                                                .dispatch(UpdateDescription {
                                                    tournament_id: tournament.tournament_id().0,
                                                    description: None,
                                                });
                                        }
                                    >
                                        // TODO: i18n once copy is approved.
                                        "Remove"
                                    </button>
                                </Show>
                            </div>
                        </Show>
                    }
                }
            >
                <ActionForm action=update attr:class="space-y-3">
                    <input
                        type="hidden"
                        name="tournament_id"
                        value=move || tournament.common.lifecycle().get().tournament_id.0
                    />
                    <input
                        type="hidden"
                        name="description"
                        prop:value=move || state.with(|state| state.draft.clone())
                    />
                    <Show
                        when=move || state.with(|state| state.previewing)
                        fallback=move || {
                            view! {
                                <textarea
                                    class="ui-field-textarea min-h-36"
                                    prop:value=move || state.with(|state| state.draft.clone())
                                    on:input=move |event| {
                                        state
                                            .update(|state| {
                                                state.draft = event_target_value(&event);
                                            });
                                    }
                                    maxlength="2000"
                                    placeholder=move || {
                                        t_string!(
                                            i18n,
                                    tournaments.detail.description_placeholder
                                        )
                                    }
                                ></textarea>
                            }
                        }
                    >
                        <div
                            class=with_class(
                                "ui-setting-group",
                                "min-h-36 w-full max-w-none break-words prose dark:prose-invert",
                            )
                            inner_html=move || {
                                state.with(|state| markdown_to_html(&state.draft))
                            }
                        />
                    </Show>
                    <Show when=move || pending.get()>
                        <p class="ui-notice" role="status">
                            {t!(i18n, tournaments.view.manage.description_pending)}
                        </p>
                    </Show>
                    <Show when=move || {
                        update.value().get().is_some_and(|result| result.is_err())
                    }>
                        <p class="ui-field-error" role="alert">
                            {t!(i18n, tournaments.view.manage.description_error)}
                        </p>
                    </Show>
                    <div class="flex flex-wrap gap-2">
                        <button
                            type="submit"
                            class="ui-button ui-button-primary ui-button-md"
                            prop:disabled=move || {
                                pending.get()
                                    || state.with(|state| state.draft.chars().count() > 2000)
                            }
                        >
                            {t!(i18n, tournaments.detail.update_description)}
                        </button>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-md"
                            on:click=move |_| {
                                update.clear();
                                state.update(DescriptionEditorState::cancel);
                            }
                        >
                            {t!(i18n, tournaments.detail.cancel)}
                        </button>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-md"
                            on:click=move |_| {
                                state.update(|state| state.previewing = !state.previewing);
                            }
                        >
                            {move || {
                                if state.with(|state| state.previewing) {
                                    t_string!(i18n, tournaments.detail.edit).to_string()
                                } else {
                                    t_string!(i18n, tournaments.detail.preview).to_string()
                                }
                            }}
                        </button>
                    </div>
                </ActionForm>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        child_route_availability,
        pre_start_boundary,
        should_retry_failed_initial_load,
        tournament_response_matches_route,
        DescriptionEditorState,
        PreStartBoundary,
        TournamentChildRoute,
    };
    use crate::providers::websocket::ConnectionReadyState;
    use chrono::{Duration, Utc};
    use shared_types::{tournament::Format, TournamentId, TournamentStatus};

    #[test]
    fn tournament_response_must_match_the_current_route() {
        let first = TournamentId("first".to_string());
        let second = TournamentId("second".to_string());

        assert!(!tournament_response_matches_route(Some(&second), &first));
        assert!(tournament_response_matches_route(Some(&second), &second));
        assert!(!tournament_response_matches_route(None, &second));
    }

    #[test]
    fn public_child_routes_follow_format_and_lifecycle() {
        assert_eq!(
            child_route_availability(
                TournamentChildRoute::Results,
                Format::RoundRobin,
                TournamentStatus::InProgress,
                false,
                false,
                true,
            ),
            Some(true),
        );
        assert_eq!(
            child_route_availability(
                TournamentChildRoute::Pairings,
                Format::RoundRobin,
                TournamentStatus::InProgress,
                false,
                false,
                true,
            ),
            Some(false),
        );
        assert_eq!(
            child_route_availability(
                TournamentChildRoute::Standings,
                Format::Swiss,
                TournamentStatus::NotStarted,
                false,
                false,
                true,
            ),
            Some(false),
        );
        assert_eq!(
            child_route_availability(
                TournamentChildRoute::Manage,
                Format::Arena,
                TournamentStatus::NotStarted,
                true,
                true,
                true,
            ),
            Some(false),
        );
    }

    #[test]
    fn scheduling_route_waits_for_identity_and_requires_active_eligibility() {
        assert_eq!(
            child_route_availability(
                TournamentChildRoute::Scheduling,
                Format::DoubleSwiss,
                TournamentStatus::InProgress,
                false,
                false,
                false,
            ),
            None,
        );
        assert_eq!(
            child_route_availability(
                TournamentChildRoute::Scheduling,
                Format::DoubleSwiss,
                TournamentStatus::Finished,
                true,
                false,
                true,
            ),
            Some(false),
        );
        assert_eq!(
            child_route_availability(
                TournamentChildRoute::Scheduling,
                Format::DoubleSwiss,
                TournamentStatus::InProgress,
                false,
                false,
                true,
            ),
            Some(false),
        );
    }

    #[test]
    fn failed_initial_load_retries_once_when_the_connection_opens() {
        assert!(should_retry_failed_initial_load(
            true,
            ConnectionReadyState::Open,
            Some(ConnectionReadyState::Closed),
        ));
        assert!(!should_retry_failed_initial_load(
            true,
            ConnectionReadyState::Open,
            Some(ConnectionReadyState::Open),
        ));
        assert!(!should_retry_failed_initial_load(
            false,
            ConnectionReadyState::Open,
            Some(ConnectionReadyState::Closed),
        ));
    }

    #[test]
    fn scheduled_start_cutoff_waits_for_the_authoritative_worker() {
        let cutoff = Utc::now();

        assert_eq!(
            pre_start_boundary(
                true,
                Format::Swiss,
                Some(cutoff),
                cutoff - Duration::milliseconds(1),
            ),
            PreStartBoundary::Open,
        );
        assert_eq!(
            pre_start_boundary(true, Format::Swiss, Some(cutoff), cutoff),
            PreStartBoundary::WaitingForScheduledWorker,
        );
        assert_eq!(
            pre_start_boundary(true, Format::Arena, Some(cutoff), cutoff),
            PreStartBoundary::Closed,
        );
    }

    #[test]
    fn description_cancel_resets_draft_and_both_editor_modes() {
        let mut state = DescriptionEditorState::new(Some("Published description".to_string()));
        state.begin_edit();
        state.draft = "Unpublished changes".to_string();
        state.previewing = true;

        state.cancel();

        assert_eq!(state.draft, "Published description");
        assert!(!state.editing);
        assert!(!state.previewing);
    }

    #[test]
    fn successful_description_update_becomes_the_new_shared_draft() {
        let mut state = DescriptionEditorState::new(Some("Old description".to_string()));
        state.begin_edit();
        state.previewing = true;

        state.complete(Some("Updated description".to_string()));

        assert_eq!(state.current.as_deref(), Some("Updated description"));
        assert_eq!(state.draft, "Updated description");
        assert!(!state.editing);
        assert!(!state.previewing);
    }
}
