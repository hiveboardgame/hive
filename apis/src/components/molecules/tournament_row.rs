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

// TODO: i18n once copy is approved.
pub fn format_tournament_clock(clock: Clock) -> String {
    match clock {
        Clock::Realtime(clock) => compact_realtime_clock(clock),
        Clock::Correspondence(clock) => {
            let (seconds, unit) = match clock {
                CorrespondenceClock::TotalTimeEach { seconds_each } => (seconds_each.get(), "side"),
                CorrespondenceClock::DaysPerMove { seconds_per_move } => {
                    (seconds_per_move.get(), "move")
                }
            };
            let duration = if seconds.is_multiple_of(86400) {
                let days = seconds / 86400;
                format!("{days} {}", if days == 1 { "day" } else { "days" })
            } else {
                format_compact_duration(seconds)
            };
            format!("{duration}/{unit}")
        }
    }
}

// TODO: i18n once copy is approved.
fn short_date(date: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let date = date.with_timezone(&Local);
    let now = now.with_timezone(&Local);
    if date.date_naive() == now.date_naive() {
        format!("Today, {}", date.format("%H:%M"))
    } else if date.year() == now.year() {
        date.format("%-d %b, %H:%M").to_string()
    } else {
        date.format("%-d %b %Y, %H:%M").to_string()
    }
}

// TODO: i18n once copy is approved.
fn entrants_text(tournament: &TournamentCardResponse) -> String {
    let players = tournament.players;
    match tournament.summary.progress {
        TournamentCardProgress::Entrants { minimum, .. } => {
            let count = tournament.seats.map_or_else(
                || format!("{players} players"),
                |capacity| format!("{players}/{capacity} players"),
            );
            if players < minimum {
                format!("{count} · {minimum} required")
            } else {
                count
            }
        }
        _ => format!("{players} players"),
    }
}

// TODO: i18n once copy is approved.
fn lifecycle_text(tournament: &TournamentCardResponse, now: DateTime<Utc>) -> (String, String) {
    let summary = &tournament.summary;
    match &summary.progress {
        TournamentCardProgress::Entrants { .. } => match summary.relevant_date {
            Some(TournamentCardDate::Starts(start)) => {
                let remaining = start - now;
                let countdown = if remaining.num_seconds() > 0 && remaining.num_minutes() < 60 {
                    format!("Starts in {}", format_time_left(remaining))
                } else if remaining.num_seconds() <= 0 {
                    String::from("Awaiting start")
                } else {
                    String::new()
                };
                (short_date(start, now), countdown)
            }
            _ => (String::from("Organizer starts"), String::new()),
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
                format!("{} remaining", format_time_left(remaining)),
                String::new(),
            )
        }
        TournamentCardProgress::Slots { resolved, total } => {
            (format!("{resolved} of {total} games"), String::new())
        }
        TournamentCardProgress::Swiss {
            completed_rounds,
            total_rounds,
            current_round,
        } => match current_round {
            Some(round) => (
                format!("Round {} of {total_rounds}", round.round),
                String::new(),
            ),
            None => (
                format!("{completed_rounds} of {total_rounds} rounds complete"),
                String::from("Between rounds"),
            ),
        },
        TournamentCardProgress::Elimination {
            decided_nodes,
            total_nodes,
        } => (
            format!("{decided_nodes} of {total_nodes} matches"),
            String::new(),
        ),
        TournamentCardProgress::Complete => (
            String::from("Completed"),
            match summary.relevant_date {
                Some(TournamentCardDate::Finished(date)) => short_date(date, now),
                _ => String::new(),
            },
        ),
    }
}

// TODO: i18n once copy is approved.
fn public_entry_text(tournament: &TournamentCardResponse) -> String {
    let summary = &tournament.summary;
    if !matches!(summary.progress, TournamentCardProgress::Entrants { .. }) {
        return String::new();
    }
    let full = tournament
        .seats
        .is_some_and(|capacity| i64::from(tournament.players) >= i64::from(capacity));
    let mut conditions = Vec::new();
    if !summary.entry_open {
        conditions.push(String::from("Entry closed"));
    } else if summary.restrictions.invite_only {
        conditions.push(String::from("Invite only"));
    } else if !full {
        conditions.push(String::from("Open entry"));
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
        BotAdmission::HumansOnly => conditions.push(String::from("Humans only")),
        BotAdmission::BotsOnly => conditions.push(String::from("Bots only")),
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
                        // TODO: i18n once copy is approved.
                        view! { <span>"Unavailable"</span> }
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
                            // TODO: i18n once copy is approved.
                            "Organizer invitation"
                        </a>
                    </Show>
                    <Show when=invitation>
                        <div class="flex relative z-20 flex-wrap gap-2 mt-2">
                            <a
                                class="ui-button ui-button-primary ui-button-sm no-link-style"
                                href=move || href.get_value()
                            >
                                // TODO: i18n once copy is approved.
                                "Review invitation"
                            </a>
                            <button
                                type="button"
                                class="ui-button ui-button-secondary ui-button-sm"
                                disabled=move || !can_decline()
                                // TODO: i18n once copy is approved.
                                aria-label=move || {
                                    format!("Decline invitation to {}", tournament.get().name)
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
                                // TODO: i18n once copy is approved.
                                "Decline"
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
