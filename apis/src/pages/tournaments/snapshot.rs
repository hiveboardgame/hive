use super::BrowseRequest;
use crate::{
    common::{ServerMessage, ServerResult, TournamentUpdate},
    components::molecules::tournament_row::TournamentRow,
    functions::tournaments::get_tournament_cards_by_ids,
    hooks::arena_clock::use_ticking_now,
    providers::{
        websocket::{ConnectionReadyState, WebsocketContext},
        AuthContext,
        AuthIdentity,
    },
    responses::{
        TournamentAccessState,
        TournamentBrowsePage,
        TournamentCardResponse,
        TournamentCategory,
        TOURNAMENT_BROWSE_PAGE_SIZE,
    },
};
use leptos::prelude::*;
use leptos_use::{use_document_visibility, use_interval_fn};
use shared_types::TournamentId;
use std::collections::{HashMap, HashSet};
use web_sys::VisibilityState;

#[derive(Clone, PartialEq)]
struct DisplayedTournament {
    card: TournamentCardResponse,
    unavailable: bool,
}

#[component]
pub(super) fn TournamentBrowseSnapshot(
    response: TournamentBrowsePage,
    request: BrowseRequest,
    current: Memo<BrowseRequest>,
    category: TournamentCategory,
    on_page_change: Callback<usize>,
) -> impl IntoView {
    let websocket = expect_context::<WebsocketContext>();
    let identity = expect_context::<AuthContext>().identity;
    let request = StoredValue::new(request);
    let is_current = move || current.get_untracked() == request.get_value();
    let page = response.page;
    let total = response.total as usize;
    // Membership and total belong to this accepted browse response. Only row values are reactive.
    let ids = StoredValue::new(
        response
            .tournaments
            .iter()
            .map(|card| card.tournament_id.clone())
            .collect::<Vec<_>>(),
    );
    let rows = RwSignal::new(
        response
            .tournaments
            .into_iter()
            .map(|card| {
                (
                    card.tournament_id.clone(),
                    DisplayedTournament {
                        card,
                        unavailable: false,
                    },
                )
            })
            .collect::<HashMap<_, _>>(),
    );
    let invitations = RwSignal::new(HashSet::<TournamentId>::new());
    let refresh = Action::new(move |_: &()| {
        let ids = ids.get_value().into_iter().collect();
        async move { get_tournament_cards_by_ids(ids).await }
    });
    let request_refresh = Callback::new(move |()| {
        if is_current() {
            refresh.dispatch(());
        }
    });
    Effect::new(move |_| {
        let Some(Ok(cards)) = refresh.value().get() else {
            return;
        };
        if !is_current() {
            return;
        }
        let updates = cards
            .into_iter()
            .map(|card| (card.tournament_id.clone(), card))
            .collect::<HashMap<_, _>>();
        rows.update(|rows| {
            for (id, displayed) in rows {
                match updates.get(id) {
                    Some(card) => {
                        displayed.card = card.clone();
                        displayed.unavailable = false;
                    }
                    None => displayed.unavailable = true,
                }
            }
        });
    });

    let visibility = use_document_visibility();
    use_interval_fn(
        move || {
            if visibility.get_untracked() == VisibilityState::Visible {
                request_refresh.run(());
            }
        },
        300_000,
    );
    let ready_state = websocket.ready_state;
    let wake_epoch = websocket.wake_resync_epoch;
    Effect::watch(
        move || (ready_state.get(), wake_epoch.get()),
        move |(ready, wake), previous, _| {
            let reopened = *ready == ConnectionReadyState::Open
                && previous.is_some_and(|(previous, _)| *previous != ConnectionReadyState::Open);
            let woke = previous.is_some_and(|(_, previous)| previous != wake);
            if reopened || woke {
                invitations.set(HashSet::new());
                request_refresh.run(());
            }
        },
        false,
    );
    Effect::watch(
        move || websocket.message.get(),
        move |message, _, _| {
            if !is_current() {
                return;
            }
            let Some(ServerResult::Ok(message)) = message else {
                return;
            };
            let ServerMessage::Tournament(update) = message.as_ref() else {
                return;
            };
            if let TournamentUpdate::InvitationFailed {
                tournament_id,
                invitee_id,
            } = update
            {
                if identity.get_untracked().and_then(AuthIdentity::user_id) == Some(*invitee_id) {
                    invitations.update(|pending| {
                        pending.remove(tournament_id);
                    });
                }
                return;
            }
            let id = match update {
                TournamentUpdate::Joined(id)
                | TournamentUpdate::Left {
                    tournament_id: id, ..
                }
                | TournamentUpdate::Declined(id)
                | TournamentUpdate::Invited(id)
                | TournamentUpdate::Uninvited(id)
                | TournamentUpdate::Deleted(id)
                | TournamentUpdate::OrganizerInvited(id)
                | TournamentUpdate::OrganizerUninvited(id)
                | TournamentUpdate::OrganizerInvitationsClosed(id)
                | TournamentUpdate::OrganizerJoined(id)
                | TournamentUpdate::OrganizerLeft(id, _)
                | TournamentUpdate::Started(id)
                | TournamentUpdate::StartDeferred(id)
                | TournamentUpdate::Finished(id) => id,
                _ => return,
            };
            if !rows.with_untracked(|rows| rows.contains_key(id)) {
                return;
            }
            if matches!(
                update,
                TournamentUpdate::Joined(_) | TournamentUpdate::Declined(_)
            ) {
                invitations.update(|pending| {
                    pending.remove(id);
                });
            }
            rows.update(|rows| {
                let Some(displayed) = rows.get_mut(id) else {
                    return;
                };
                match update {
                    TournamentUpdate::Joined(_) => {
                        displayed.card.invited = false;
                        displayed.card.summary.access = Some(TournamentAccessState::Joined);
                    }
                    TournamentUpdate::Declined(_) | TournamentUpdate::Uninvited(_) => {
                        displayed.card.invited = false;
                        if displayed.card.summary.access
                            == Some(TournamentAccessState::PendingInvitation)
                        {
                            displayed.card.summary.access = None;
                        }
                    }
                    TournamentUpdate::Deleted(_) => displayed.unavailable = true,
                    _ => (),
                }
            });
            request_refresh.run(());
        },
        false,
    );
    let on_invitation = Callback::new(move |id| {
        invitations.update(|pending| {
            pending.insert(id);
        })
    });
    let now = use_ticking_now();

    view! {
        <div class="space-y-3">
            <BrowsePagination page total on_page_change />
            <div class="overflow-hidden rounded-lg border divide-y border-black/10 divide-black/10 dark:border-white/10 dark:divide-white/10">
                <For
                    each=move || ids.get_value()
                    key=|id| id.clone()
                    children=move |id| {
                        let id = StoredValue::new(id);
                        let displayed = Memo::new(move |_| {
                            rows.with(|rows| rows[&id.get_value()].clone())
                        });
                        let tournament = Signal::derive(move || {
                            displayed.with(|row| row.card.clone())
                        });
                        let unavailable = Signal::derive(move || {
                            displayed.with(|row| row.unavailable)
                        });
                        let invitation_pending = Signal::derive(move || {
                            invitations.with(|pending| pending.contains(&id.get_value()))
                        });
                        view! {
                            <TournamentRow
                                tournament
                                now
                                unavailable
                                invitation_pending
                                on_invitation
                                category
                            />
                        }
                    }
                />
            </div>
            <BrowsePagination page total on_page_change />
        </div>
    }
}

#[component]
fn BrowsePagination(page: usize, total: usize, on_page_change: Callback<usize>) -> impl IntoView {
    let pages = total.div_ceil(TOURNAMENT_BROWSE_PAGE_SIZE).max(1);
    let start = ((page - 1) * TOURNAMENT_BROWSE_PAGE_SIZE + 1).min(total);
    let end = (page * TOURNAMENT_BROWSE_PAGE_SIZE).min(total);
    // Keep comparisons outside attributes: leptosfmt strips braces needed around `>=`.
    let previous_disabled = page <= 1;
    let next_disabled = page >= pages;
    view! {
        // TODO: i18n once copy is approved.
        <nav
            class="flex flex-wrap gap-y-2 gap-x-4 justify-between items-center text-sm"
            aria-label="Tournament pagination"
        >
            <p class="text-gray-600 dark:text-gray-300">
                {format!("Showing {start}–{end} of {total} tournaments")}
            </p>
            <div class="flex gap-2 items-center">
                <button
                    type="button"
                    class="ui-button ui-button-secondary ui-button-sm"
                    disabled=previous_disabled
                    on:click=move |_| {
                        if page > 1 {
                            on_page_change.run(page - 1);
                        }
                    }
                >
                    "Previous"
                </button>
                <span class="text-xs tabular-nums text-gray-600 dark:text-gray-300">
                    {format!("Page {page} of {pages}")}
                </span>
                <button
                    type="button"
                    class="ui-button ui-button-secondary ui-button-sm"
                    disabled=next_disabled
                    on:click=move |_| {
                        if page < pages {
                            on_page_change.run(page + 1);
                        }
                    }
                >
                    "Next"
                </button>
            </div>
        </nav>
    }
}
