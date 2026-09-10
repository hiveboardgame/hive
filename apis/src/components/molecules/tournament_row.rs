use crate::{
    common::{format_local_datetime, tournament_format_label, TournamentAction},
    components::molecules::time_row::TimeRow,
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext, AuthIdentity},
    responses::{
        TournamentCardDate,
        TournamentCardProgress,
        TournamentCardResponse,
        TournamentCategory,
    },
};
use chrono::{DateTime, Utc};
use leptos::prelude::*;

// TODO: i18n once copy is approved.
fn entrants_text(players: u32, capacity: Option<i32>) -> String {
    capacity.map_or_else(
        || format!("{players} entrants"),
        |capacity| format!("{players} / {capacity} entrants"),
    )
}

// TODO: i18n once copy is approved.
fn lifecycle_time_text(locale: Locale, date: Option<TournamentCardDate>) -> String {
    match date {
        Some(TournamentCardDate::Starts(date)) => {
            format!("Scheduled {}", format_local_datetime(locale, date))
        }
        Some(TournamentCardDate::Started(date)) => {
            format!("Started {}", format_local_datetime(locale, date))
        }
        Some(TournamentCardDate::Finished(date)) => {
            format!("Completed {}", format_local_datetime(locale, date))
        }
        None => String::from("Organizer starts"),
    }
}

fn live_arena_elapsed(
    elapsed_seconds: u32,
    duration_seconds: u32,
    started_at: Option<DateTime<Utc>>,
    now: Option<DateTime<Utc>>,
) -> u32 {
    started_at
        .zip(now)
        .map(|(started_at, now)| (now - started_at).num_seconds().max(0))
        .and_then(|seconds| u32::try_from(seconds).ok())
        .unwrap_or(elapsed_seconds)
        .min(duration_seconds)
}

// TODO: i18n once copy is approved.
fn progress_text(progress: &TournamentCardProgress, arena_elapsed_seconds: Option<u32>) -> String {
    match progress {
        TournamentCardProgress::Entrants { current, minimum } => {
            if current >= minimum {
                String::from("Ready to start")
            } else {
                format!("{current} / {minimum} minimum")
            }
        }
        TournamentCardProgress::Arena {
            elapsed_seconds,
            duration_seconds,
        } => format!(
            "{} / {} min",
            arena_elapsed_seconds.unwrap_or(*elapsed_seconds) / 60,
            duration_seconds / 60,
        ),
        TournamentCardProgress::Slots { resolved, total } => {
            format!("{resolved} / {total} games")
        }
        TournamentCardProgress::Swiss {
            completed_rounds,
            total_rounds,
            current_round,
        } => match current_round {
            Some(current_round) => {
                format!(
                    "Round {} / {total_rounds} · {} / {} pairings",
                    current_round.round,
                    current_round.resolved_encounters,
                    current_round.total_encounters,
                )
            }
            None => format!("{completed_rounds} / {total_rounds} rounds"),
        },
        TournamentCardProgress::Elimination {
            decided_nodes,
            total_nodes,
        } => format!("{decided_nodes} / {total_nodes} matches"),
        TournamentCardProgress::Complete => String::from("Completed"),
    }
}

#[component]
pub fn TournamentRow(
    tournament: TournamentCardResponse,
    #[prop(optional)] category: Option<TournamentCategory>,
) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth = expect_context::<AuthContext>();
    let viewer = auth.identity.get_untracked();
    let tournament_id = StoredValue::new(tournament.tournament_id.clone());
    let invitation_row = category == Some(TournamentCategory::Invitations) && tournament.invited;
    let organizer_invitation = tournament.organizer_invited;
    let i18n = use_i18n();
    let format = tournament.format();
    let primary_clock = tournament.summary.primary_clock;
    let progress = tournament.summary.progress.clone();
    let progress_for_text = progress.clone();
    let relevant_date = tournament.summary.relevant_date;
    let started_at = match relevant_date {
        Some(TournamentCardDate::Started(started_at)) => Some(started_at),
        _ => None,
    };
    let ticking_now = if matches!(progress, TournamentCardProgress::Arena { .. }) {
        Some(use_ticking_now())
    } else {
        None
    };
    let arena_elapsed = Memo::new(move |_| match &progress {
        TournamentCardProgress::Arena {
            elapsed_seconds,
            duration_seconds,
        } => Some(live_arena_elapsed(
            *elapsed_seconds,
            *duration_seconds,
            started_at,
            ticking_now.map(|now| now.get()),
        )),
        _ => None,
    });
    let progress_label = move || {
        if matches!(progress_for_text, TournamentCardProgress::Complete)
            && matches!(relevant_date, Some(TournamentCardDate::Finished(_)))
        {
            String::new()
        } else {
            progress_text(&progress_for_text, arena_elapsed.get())
        }
    };
    let access = tournament.summary.access;
    let tournament_href = format!("/tournament/{}", tournament.tournament_id.0);
    let name_for_link = tournament.name.clone();

    view! {
        <article class="grid relative grid-cols-2 gap-y-2 gap-x-3 items-center p-3 min-w-0 text-sm sm:px-4 ui-card-row lg:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)_minmax(0,0.65fr)_minmax(0,0.9fr)_minmax(0,1.1fr)_minmax(0,1.1fr)]">
            <h2 class="col-span-2 min-w-0 font-bold text-gray-900 break-words lg:col-span-1 dark:text-gray-100">
                {tournament.name}
            </h2>
            <div class="flex flex-wrap col-span-2 gap-y-1 gap-x-2 min-w-0 text-gray-600 lg:flex-col lg:col-span-1 dark:text-gray-300">
                <span>{move || tournament_format_label(i18n, format)}</span>
                {primary_clock.map(|clock| view! { <TimeRow time_control=Some(clock) /> })}
            </div>
            <span class="min-w-0 text-gray-600 dark:text-gray-300">
                {entrants_text(tournament.players, tournament.seats)}
            </span>
            <span class="min-w-0 text-gray-600 dark:text-gray-300">{progress_label}</span>
            <span class="min-w-0 text-xs text-gray-600 break-words dark:text-gray-300">
                {move || lifecycle_time_text(i18n.get_locale(), relevant_date)}
            </span>
            <div class="flex relative z-20 flex-wrap gap-2 items-center min-w-0">
                {access
                    .map(|access| {
                        view! { <span class="text-xs font-medium">{access.label()}</span> }
                    })} <Show when=move || organizer_invitation>
                    <a
                        class="ui-button ui-button-secondary ui-button-sm no-link-style"
                        href=format!("/tournament/{}", tournament_id.get_value())
                    >
                        // TODO: i18n once copy is approved.
                        "Organizer invitation"
                    </a>
                </Show>
                <Show when=move || {
                    invitation_row
                }>
                    {move || {
                        let current_viewer = auth.identity.get();
                        let can_act = current_viewer == viewer
                            && matches!(current_viewer, Some(AuthIdentity::User(_)));
                        if can_act && access.is_some_and(|access| access.can_enter()) {
                            view! {
                                <button
                                    type="button"
                                    class="ui-button ui-button-primary ui-button-sm"
                                    on:click=move |_| {
                                        if auth.identity.get_untracked() == viewer {
                                            api.get()
                                                .tournament(
                                                    TournamentAction::InvitationAccept(
                                                        tournament_id.get_value(),
                                                    ),
                                                );
                                        }
                                    }
                                >
                                    // TODO: i18n once copy is approved.
                                    "Accept"
                                </button>
                                <button
                                    type="button"
                                    class="ui-button ui-button-secondary ui-button-sm"
                                    on:click=move |_| {
                                        if auth.identity.get_untracked() == viewer {
                                            api.get()
                                                .tournament(
                                                    TournamentAction::InvitationDecline(
                                                        tournament_id.get_value(),
                                                    ),
                                                );
                                        }
                                    }
                                >
                                    // TODO: i18n once copy is approved.
                                    "Decline"
                                </button>
                            }
                                .into_any()
                        } else {
                            view! {
                                <a
                                    class="ui-button ui-button-secondary ui-button-sm no-link-style"
                                    href=format!("/tournament/{}", tournament_id.get_value())
                                >
                                    // TODO: i18n once copy is approved.
                                    "View details"
                                </a>
                            }
                                .into_any()
                        }
                    }}
                </Show>
            </div>
            <a
                class="absolute inset-0 z-10 rounded-lg"
                href=tournament_href
                aria-label=name_for_link
            ></a>
        </article>
    }
}
