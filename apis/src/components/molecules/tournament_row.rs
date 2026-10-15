use crate::{
    common::{compact_realtime_clock, tournament_format_label, TournamentAction},
    components::molecules::time_row::format_compact_duration,
    hooks::{arena_clock::format_time_left, tap_feedback::use_tap_feedback},
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext, AuthIdentity},
    responses::{
        TournamentAccessState,
        TournamentCardDate,
        TournamentCardProgress,
        TournamentCardResponse,
        TournamentCategory,
    },
};
use chrono::{DateTime, Datelike, Duration, Local, Utc};
use leptos::prelude::*;
use shared_types::{tournament::BotAdmission, Clock, CorrespondenceClock, TournamentId};
pub fn format_tournament_clock(clock: Clock) -> String {
    let i18n = use_i18n();
    match clock {
        Clock::Realtime(clock) => compact_realtime_clock(clock),
        Clock::Correspondence(clock) => {
            let (seconds, per_move) = match clock {
                CorrespondenceClock::TotalTimeEach { seconds_each } => (seconds_each.get(), false),
                CorrespondenceClock::DaysPerMove { seconds_per_move } => {
                    (seconds_per_move.get(), true)
                }
            };
            let duration = if seconds.is_multiple_of(86400) {
                let days = seconds / 86400;
                if days == 1 {
                    t_string!(i18n, tournaments.row.one_day).to_string()
                } else {
                    t_string!(
                        i18n,
                        home.custom_game.mode.correspondence.time,
                        count = days
                    )
                    .to_string()
                }
            } else {
                format_compact_duration(seconds)
            };
            if per_move {
                t_string!(i18n, tournaments.row.duration_per_move, duration = duration).to_string()
            } else {
                t_string!(i18n, tournaments.row.duration_per_side, duration = duration).to_string()
            }
        }
    }
}
fn short_date(date: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let i18n = use_i18n();
    let date = date.with_timezone(&Local);
    let now = now.with_timezone(&Local);
    if date.date_naive() == now.date_naive() {
        t_string!(
            i18n,
            tournaments.row.today_value1,
            value1 = (date.format("%H:%M"))
        )
        .to_string()
    } else if date.year() == now.year() {
        t_string!(
            i18n,
            common.short_datetime,
            month = date.format("%m").to_string(),
            day = date.format("%d").to_string(),
            time = date.format("%H:%M").to_string()
        )
        .to_string()
    } else {
        t_string!(
            i18n,
            common.date_time,
            year = date.format("%Y").to_string(),
            month = date.format("%m").to_string(),
            day = date.format("%d").to_string(),
            time = date.format("%H:%M").to_string()
        )
        .to_string()
    }
}
fn entrants_text(tournament: &TournamentCardResponse) -> String {
    let i18n = use_i18n();
    let players = tournament.players;
    match tournament.summary.progress {
        TournamentCardProgress::Entrants { minimum, .. } => {
            let count = tournament.seats.map_or_else(
                || {
                    t_string!(i18n, tournaments.row.players_players, players = (players))
                        .to_string()
                },
                |capacity| {
                    t_string!(
                        i18n,
                        tournaments.row.players_capacity_players,
                        players = (players),
                        capacity = (capacity)
                    )
                    .to_string()
                },
            );
            if players < minimum {
                t_string!(
                    i18n,
                    tournaments.row.count_minimum_required,
                    count = (count),
                    minimum = (minimum)
                )
                .to_string()
            } else {
                count
            }
        }
        _ => t_string!(i18n, tournaments.row.players_players, players = (players)).to_string(),
    }
}
fn lifecycle_text(tournament: &TournamentCardResponse, now: DateTime<Utc>) -> (String, String) {
    let i18n = use_i18n();
    let summary = &tournament.summary;
    match &summary.progress {
        TournamentCardProgress::Entrants { .. } => match summary.relevant_date {
            Some(TournamentCardDate::Starts(start)) => {
                let remaining = start - now;
                let countdown = if remaining.num_seconds() > 0 && remaining.num_minutes() < 60 {
                    t_string!(
                        i18n,
                        tournaments.row.starts_in_value1,
                        value1 = (format_time_left(i18n, remaining))
                    )
                    .to_string()
                } else if remaining.num_seconds() <= 0 {
                    t_string!(i18n, tournaments.row.awaiting_start).to_string()
                } else {
                    String::new()
                };
                (short_date(start, now), countdown)
            }
            _ => (
                t_string!(i18n, tournaments.row.organizer_starts).to_string(),
                String::new(),
            ),
        },
        TournamentCardProgress::Arena {
            elapsed_seconds,
            duration_seconds,
        } => {
            let remaining = summary.arena_ends_at.map_or_else(
                || Duration::seconds(i64::from(duration_seconds.saturating_sub(*elapsed_seconds))),
                |end| end - now,
            );
            (
                t_string!(
                    i18n,
                    tournaments.row.value1_remaining,
                    value1 = (format_time_left(i18n, remaining))
                )
                .to_string(),
                String::new(),
            )
        }
        TournamentCardProgress::Slots { resolved, total } => (
            t_string!(
                i18n,
                tournaments.row.resolved_of_total_games,
                resolved = (resolved),
                total = (total)
            )
            .to_string(),
            String::new(),
        ),
        TournamentCardProgress::Swiss {
            completed_rounds,
            total_rounds,
            current_round,
        } => match current_round {
            Some(round) => (
                t_string!(
                    i18n,
                    tournaments.row.round_value1_of_total_rounds,
                    value1 = (round.round),
                    total_rounds = (total_rounds)
                )
                .to_string(),
                String::new(),
            ),
            None => (
                t_string!(
                    i18n,
                    tournaments
                        .row
                        .completed_rounds_of_total_rounds_rounds_complete,
                    completed_rounds = (completed_rounds),
                    total_rounds = (total_rounds)
                )
                .to_string(),
                t_string!(i18n, tournaments.row.between_rounds).to_string(),
            ),
        },
        TournamentCardProgress::Elimination {
            decided_nodes,
            total_nodes,
        } => (
            t_string!(
                i18n,
                tournaments.row.decided_nodes_of_total_nodes_matches,
                decided_nodes = (decided_nodes),
                total_nodes = (total_nodes)
            )
            .to_string(),
            String::new(),
        ),
        TournamentCardProgress::Complete => (
            t_string!(i18n, tournaments.row.completed).to_string(),
            match summary.relevant_date {
                Some(TournamentCardDate::Finished(date)) => short_date(date, now),
                _ => String::new(),
            },
        ),
    }
}
fn public_entry_text(tournament: &TournamentCardResponse) -> String {
    let i18n = use_i18n();
    let summary = &tournament.summary;
    if !matches!(summary.progress, TournamentCardProgress::Entrants { .. }) {
        return String::new();
    }
    let full = tournament
        .seats
        .is_some_and(|capacity| i64::from(tournament.players) >= i64::from(capacity));
    let mut conditions = Vec::new();
    if !summary.entry_open {
        conditions.push(t_string!(i18n, tournaments.admission.closed).to_string());
    } else if summary.restrictions.invite_only {
        conditions.push(t_string!(i18n, tournaments.creation.invite_only).to_string());
    } else if !full {
        conditions.push(t_string!(i18n, tournaments.detail.open_entry).to_string());
    }
    match (
        summary.restrictions.band_lower,
        summary.restrictions.band_upper,
    ) {
        (Some(lower), Some(upper)) => conditions.push(format!("{lower}–{upper} Elo")),
        (Some(lower), None) => conditions.push(format!("≥{lower} Elo")),
        (None, Some(upper)) => conditions.push(format!("≤{upper} Elo")),
        (None, None) => (),
    }
    match summary.bot_admission {
        BotAdmission::HumansOnly => {
            conditions.push(t_string!(i18n, tournaments.row.humans_only).to_string())
        }
        BotAdmission::BotsOnly => {
            conditions.push(t_string!(i18n, tournaments.row.bots_only).to_string())
        }
        BotAdmission::HumansAndBots => (),
    }
    conditions.join(" · ")
}

#[component]
pub fn TournamentRow(
    tournament: Signal<TournamentCardResponse>,
    now: Signal<DateTime<Utc>>,
    unavailable: Signal<bool>,
    invitation_pending: Signal<bool>,
    on_invitation: Callback<TournamentId>,
    category: TournamentCategory,
) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth = expect_context::<AuthContext>();
    let viewer = auth.identity.get_untracked();
    let i18n = use_i18n();
    let id = StoredValue::new(tournament.get_untracked().tournament_id);
    let href = StoredValue::new(format!("/tournament/{}", id.get_value()));
    let lifecycle = Memo::new(move |_| lifecycle_text(&tournament.get(), now.get()));
    let entry = Memo::new(move |_| public_entry_text(&tournament.get()));
    let viewer_label = move || {
        if auth.identity.get() != viewer {
            return String::new();
        }
        match tournament.get().summary.access {
            Some(
                TournamentAccessState::Open
                | TournamentAccessState::Full
                | TournamentAccessState::EntryClosed
                | TournamentAccessState::InvitationRequired
                | TournamentAccessState::AuthPending,
            )
            | None => String::new(),
            Some(access) => access.label(),
        }
    };
    let invitation =
        move || category == TournamentCategory::Invitations && tournament.get().invited;
    let can_decline = move || {
        !unavailable.get()
            && !invitation_pending.get()
            && tournament.get().summary.entry_open
            && auth.identity.get() == viewer
            && matches!(viewer, Some(AuthIdentity::User(_)))
    };
    let mark_pressed = use_tap_feedback("[data-tournament-interactive]");

    view! {
        <article
            class="grid relative grid-cols-2 gap-y-3 gap-x-4 items-center py-4 px-4 min-w-0 text-sm bg-white [&[data-tournament-interactive]:hover]:bg-blue-light/70 data-[ui-pressed=true]:bg-blue-light/70 md:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,0.9fr)] dark:bg-surface-card dark:[&[data-tournament-interactive]:hover]:bg-pillbug-teal/15 dark:data-[ui-pressed=true]:bg-pillbug-teal/15"
            data-tournament-id=move || id.get_value().0
            data-tournament-interactive=move || (!unavailable.get()).then_some("")
            on:pointerdown=move |event| mark_pressed.run(event)
        >
            <div class="col-span-2 min-w-0 md:col-span-1">
                <h2 class="font-bold text-gray-900 break-words dark:text-gray-100">
                    {move || tournament.get().name}
                </h2>
                <div class="mt-1 text-gray-600 dark:text-gray-300">
                    {move || tournament_format_label(i18n, tournament.get().summary.format)}
                    {move || {
                        tournament
                            .get()
                            .summary
                            .primary_clock
                            .map(|clock| format!(" · {}", format_tournament_clock(clock)))
                    }}
                </div>
            </div>
            <div class="min-w-0 text-gray-900 dark:text-gray-100">
                <Show
                    when=move || !unavailable.get()
                    fallback=move || {
                        view! { <span>{t!(i18n, common.unavailable)}</span> }
                    }
                >
                    <p class="tabular-nums">{move || lifecycle.get().0}</p>
                    <Show when=move || !lifecycle.get().1.is_empty()>
                        <p class="mt-1 text-xs text-gray-600 dark:text-gray-300">
                            {move || lifecycle.get().1}
                        </p>
                    </Show>
                </Show>
            </div>
            <div class="min-w-0 text-gray-700 dark:text-gray-200">
                <p>{move || entrants_text(&tournament.get())}</p>
                <Show when=move || !unavailable.get()>
                    <Show when=move || !entry.get().is_empty()>
                        <p class="mt-1 text-xs text-gray-600 dark:text-gray-300">{entry}</p>
                    </Show>
                    <Show when=move || !viewer_label().is_empty()>
                        <p class="mt-1 text-xs font-medium text-teal-700 dark:text-teal-300">
                            {viewer_label}
                        </p>
                    </Show>
                    <Show when=move || tournament.get().organizer_invited>
                        <a
                            class="relative z-20 mt-2 ui-button ui-button-secondary ui-button-sm no-link-style"
                            href=move || href.get_value()
                        >
                            {t!(i18n, tournaments.ui.organizer_invitation)}
                        </a>
                    </Show>
                    <Show when=invitation>
                        <div class="flex relative z-20 flex-wrap gap-2 mt-2">
                            <a
                                class="ui-button ui-button-primary ui-button-sm no-link-style"
                                href=move || href.get_value()
                            >
                                {t!(i18n, tournaments.row.review_invitation)}
                            </a>
                            <button
                                type="button"
                                class="ui-button ui-button-secondary ui-button-sm"
                                disabled=move || !can_decline()
                                aria-label=move || {
                                    t_string!(
                                        i18n, tournaments.row.decline_invitation_to_value1, value1 = (tournament.get().name)
                                    )
                                        .to_string()
                                }
                                on:click=move |_| {
                                    if can_decline()
                                        && api
                                            .get()
                                            .tournament(
                                                TournamentAction::InvitationDecline(id.get_value()),
                                            )
                                    {
                                        on_invitation.run(id.get_value());
                                    }
                                }
                            >
                                {t!(i18n, common.decline)}
                            </button>
                        </div>
                    </Show>
                </Show>
            </div>
            <Show when=move || !unavailable.get()>
                <a
                    class="absolute inset-0 z-10 rounded focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-pillbug-teal"
                    href=move || href.get_value()
                    aria-label=move || tournament.get().name
                ></a>
            </Show>
        </article>
    }
}
