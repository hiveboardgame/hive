use crate::{
    common::{
        format_local_datetime,
        tournament_path_matches,
        TournamentAction,
        TournamentCloseoutIntent,
    },
    components::{
        molecules::{modal::Modal, pagination_controls::PaginationControls},
        organisms::tournament_slots::{slot_card_label, slot_ordering},
    },
    i18n::*,
    providers::{
        ApiRequestsProvider,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommonStoreFields,
        TournamentFormatState,
        TournamentState,
    },
};
use chrono::{DateTime, Local, Utc};
use hive_lib::GameStatus;
use leptos::{html::Dialog, prelude::*};
use leptos_router::hooks::use_location;
use shared_types::{tournament_view::SlotResponse, SlotAdminAction, TournamentId};
use uuid::Uuid;

const REVIEW_PAGE_SIZE: usize = 6;

fn slot_is_closeout_eligible(slot: &SlotResponse) -> bool {
    slot.resolution.is_none()
        && match &slot.game {
            Some(game) => !game.finished && game.status == GameStatus::NotStarted,
            None => slot
                .available_admin_actions
                .contains(&SlotAdminAction::RecordResult),
        }
}

#[derive(Clone)]
struct ReviewedCloseoutSlot {
    id: Uuid,
    order: (u8, usize, usize, usize, usize),
    slot: SlotResponse,
    players: [Option<String>; 2],
    scheduled_at: Option<DateTime<Utc>>,
}

fn reviewed_closeout_slots(tournament: TournamentState) -> Vec<ReviewedCloseoutSlot> {
    let slot_ids = match tournament.format {
        TournamentFormatState::RoundRobin(state) => state
            .slots()
            .with_untracked(|slots| slots.keys().copied().collect::<Vec<_>>()),
        TournamentFormatState::Swiss(state) => state
            .slots()
            .with_untracked(|slots| slots.keys().copied().collect::<Vec<_>>()),
        _ => return Vec::new(),
    };
    let slot_order = slot_ordering(tournament);
    let memberships = tournament.common.memberships().get_untracked();
    let mut rows = slot_ids
        .into_iter()
        .filter_map(|id| {
            let slot = tournament.format.slot(id)?.try_get_untracked()?;
            if !slot_is_closeout_eligible(&slot) {
                return None;
            }

            let players = [slot.white(), slot.black()].map(|player| {
                memberships
                    .players
                    .get(&player)
                    .map(|user| user.username.clone())
            });
            Some(ReviewedCloseoutSlot {
                id,
                order: slot_order(&slot),
                slot: slot.clone(),
                players,
                scheduled_at: slot.scheduled_at,
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.order.cmp(&right.order).then(left.id.cmp(&right.id)));
    rows
}

fn current_tournament_id(
    pathname: Memo<String>,
    tournament: TournamentState,
    expected_tournament_id: StoredValue<TournamentId>,
) -> Option<TournamentId> {
    let pathname = pathname.try_get()?;
    let expected_tournament_id = expected_tournament_id.try_get_value()?;
    if !tournament_path_matches(&pathname, &expected_tournament_id) {
        return None;
    }
    tournament
        .common
        .lifecycle()
        .try_get()
        .and_then(|lifecycle| {
            (lifecycle.tournament_id == expected_tournament_id).then_some(lifecycle.tournament_id)
        })
}

#[component]
pub fn TournamentCloseout(tournament: TournamentState, organizer: Signal<bool>) -> impl IntoView {
    let i18n = use_i18n();
    let reviewed = RwSignal::new(None::<Vec<ReviewedCloseoutSlot>>);
    let reviewed_count =
        Signal::derive(move || reviewed.with(|rows| rows.as_ref().map_or(0, Vec::len)));
    let page = RwSignal::new(1usize);
    let dialog_el = NodeRef::<Dialog>::new();
    let api = expect_context::<ApiRequestsProvider>().0;
    let pathname = use_location().pathname;
    let expected_tournament_id = StoredValue::new(tournament.tournament_id());
    let route_active = Signal::derive(move || {
        organizer.try_get() == Some(true)
            && current_tournament_id(pathname, tournament, expected_tournament_id).is_some()
    });
    let closeout_eligible_slots = Signal::derive(move || match tournament.format {
        TournamentFormatState::RoundRobin(state) => state.closeout_eligible_slots().get(),
        TournamentFormatState::Swiss(state) => state.closeout_eligible_slots().get(),
        _ => 0,
    });
    let cancel = Callback::new(move |_: ()| {
        if let Some(dialog) = dialog_el.try_get().flatten() {
            dialog.close();
        }
        let _ = reviewed.try_set(None);
    });
    Effect::new(move |_| {
        if !route_active.get() {
            cancel.run(());
        }
    });
    let confirm = Callback::new(move |_: ()| {
        if !route_active.get_untracked() {
            return;
        }
        let Some(tournament_id) =
            current_tournament_id(pathname, tournament, expected_tournament_id)
        else {
            return;
        };
        let Some(rows) = reviewed.get_untracked() else {
            return;
        };
        // The command comes only from the frozen rows the organizer reviewed.
        let slot_ids = rows.into_iter().map(|row| row.id).collect();
        api.get().tournament(TournamentAction::CloseUnstarted(
            tournament_id,
            TournamentCloseoutIntent { slot_ids },
        ));
        cancel.run(());
    });
    view! {
        <Show when=move || { route_active.get() && closeout_eligible_slots.get() > 0 }>
            <button
                type="button"
                class="ui-button ui-button-secondary ui-button-sm"
                data-testid="tournament-closeout"
                on:click=move |_| {
                    if !route_active.get_untracked() {
                        return;
                    }
                    let rows = reviewed_closeout_slots(tournament);
                    if rows.is_empty() {
                        return;
                    }
                    page.set(1);
                    reviewed.set(Some(rows));
                    if let Some(dialog) = dialog_el.try_get().flatten() {
                        let _ = dialog.show_modal();
                    }
                }
            >

                {move || {
                    t_string!(
                        i18n, tournaments.ui.close_unstarted_games, value1 = closeout_eligible_slots.get()
                    )
                        .to_string()
                }}
            </button>
        </Show>

        <Modal
            dialog_el
            aria_label=move || {
                t_string!(i18n, tournaments.ui.close_unstarted_games_title).to_string()
            }
            on_close=cancel
        >
            <Show when=move || route_active.get() && reviewed.with(Option::is_some)>
                <div class="px-3 pb-4 mx-auto space-y-3 sm:px-4 w-[min(94vw,48rem)]">
                    <header>

                        <h2 class="text-xl font-bold">
                            {move || {
                                let count = reviewed_count.get();
                                t_string!(i18n, tournaments.ui.close_unstarted, count = count)
                                    .to_string()
                            }}
                        </h2>
                        <p class="text-sm text-gray-600 dark:text-gray-300">
                            {move || tournament.common.lifecycle().get().name}
                        </p>
                    </header>

                    <p class="p-3 text-sm ui-danger-notice">
                        {move || {
                            t_string!(
                                i18n, tournaments.ui.close_games_double_forfeit_help, count = reviewed_count.get()
                            )
                                .to_string()
                        }}
                    </p>

                    <p class="text-xs text-gray-600 dark:text-gray-300">
                        {move || {
                            t_string!(
                                i18n, tournaments.ui.your_local_time_currently, value1 = Local::now().format("UTC%:z")
                            )
                                .to_string()
                        }}
                    </p>
                    <div class="overflow-hidden rounded border divide-y border-black/10 divide-black/10 dark:border-white/10 dark:divide-white/10">

                        <div class="hidden gap-3 p-3 text-xs font-semibold sm:grid sm:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1.3fr)]">
                            <span>{t!(i18n, common.players)}</span>
                            <span>{t!(i18n, common.game)}</span>
                            <span>{t!(i18n, tournaments.ui.agreed_time)}</span>
                        </div>
                        <For
                            each=move || {
                                reviewed
                                    .with(|rows| {
                                        rows.as_ref()
                                            .map(|rows| {
                                                rows.iter()
                                                    .skip((page.get() - 1) * REVIEW_PAGE_SIZE)
                                                    .take(REVIEW_PAGE_SIZE)
                                                    .cloned()
                                                    .collect::<Vec<_>>()
                                            })
                                            .unwrap_or_default()
                                    })
                            }
                            key=|row| row.id
                            let:row
                        >
                            <article
                                class="grid gap-1 p-3 text-sm sm:gap-3 sm:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1.3fr)]"
                                data-slot-id=row.id.to_string()
                            >
                                <p class="font-semibold break-words">
                                    {move || {
                                        let white = row
                                            .players[0]
                                            .clone()
                                            .unwrap_or_else(|| {
                                                t_string!(i18n, common.white).to_string()
                                            });
                                        let black = row
                                            .players[1]
                                            .clone()
                                            .unwrap_or_else(|| {
                                                t_string!(i18n, common.black).to_string()
                                            });
                                        t_string!(
                                            i18n, common.versus_players, white = white, black = black
                                        )
                                            .to_string()
                                    }}
                                </p>
                                <p>
                                    {move || {
                                        slot_card_label(i18n, tournament, &row.slot)
                                            .unwrap_or_else(|| t_string!(i18n, common.game).to_string())
                                    }}
                                </p>

                                <p class="text-gray-600 dark:text-gray-300">
                                    {move || {
                                        row.scheduled_at
                                            .map(|time| format_local_datetime(i18n.get_locale(), time))
                                            .unwrap_or_else(|| {
                                                t_string!(i18n, tournaments.ui.no_agreed_time).to_string()
                                            })
                                    }}
                                </p>
                            </article>
                        </For>
                    </div>
                    <Show when=move || { reviewed_count.get() > REVIEW_PAGE_SIZE }>
                        <PaginationControls
                            page=page.into()
                            total=reviewed_count
                            page_size=REVIEW_PAGE_SIZE
                            on_page_change=Callback::new(move |next| page.set(next))
                        />
                    </Show>
                    <footer class="flex flex-wrap gap-2 justify-end pt-3 border-t border-black/10 dark:border-white/10">
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-md"
                            on:click=move |_| cancel.run(())
                        >
                            {t!(i18n, tournaments.detail.cancel)}
                        </button>

                        <button
                            type="button"
                            class="ui-button ui-button-danger ui-button-md"
                            on:click=move |_| confirm.run(())
                        >
                            {move || {
                                t_string!(
                                    i18n, tournaments.ui.record_double_forfeits, count = reviewed_count.get()
                                )
                                    .to_string()
                            }}
                        </button>
                    </footer>
                </div>
            </Show>
        </Modal>
    }
}
