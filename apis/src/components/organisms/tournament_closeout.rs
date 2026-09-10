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
        TournamentFormatStore,
        TournamentState,
    },
};
use chrono::{DateTime, Local, Utc};
use hive_lib::GameStatus;
use leptos::{html::Dialog, prelude::*};
use leptos_i18n::I18nContext;
use leptos_router::hooks::use_location;
use shared_types::{tournament_view::SlotResponse, TournamentId};
use uuid::Uuid;

fn slot_is_closeout_eligible(slot: &SlotResponse, gameless: bool) -> bool {
    slot.resolution.is_none()
        && match &slot.game {
            Some(game) => !game.finished && game.status == GameStatus::NotStarted,
            None => gameless,
        }
}

#[derive(Clone)]
struct ReviewedCloseoutSlot {
    id: Uuid,
    order: (u8, usize, usize, usize, usize),
    identity: String,
    players: String,
    scheduled_at: Option<DateTime<Utc>>,
    released: bool,
}

fn reviewed_closeout_slots(
    tournament: TournamentState,
    i18n: I18nContext<Locale, I18nKeys>,
) -> Vec<ReviewedCloseoutSlot> {
    let (slot_ids, gameless) = match tournament.format {
        TournamentFormatStore::RoundRobin(state) => (
            state
                .slots()
                .with_untracked(|slots| slots.keys().copied().collect::<Vec<_>>()),
            true,
        ),
        TournamentFormatStore::Swiss(state) => (
            state
                .slots()
                .with_untracked(|slots| slots.keys().copied().collect::<Vec<_>>()),
            false,
        ),
        _ => return Vec::new(),
    };
    let slot_order = slot_ordering(tournament);
    let memberships = tournament.common.memberships().get_untracked();
    let mut rows = slot_ids
        .into_iter()
        .filter_map(|id| {
            let slot = tournament.format.slot(id)?.try_get_untracked()?;
            if !slot_is_closeout_eligible(&slot, gameless) {
                return None;
            }
            // TODO: i18n once copy is approved.
            let white = memberships
                .players
                .get(&slot.white())
                .map(|user| user.username.as_str())
                .unwrap_or("White");
            let black = memberships
                .players
                .get(&slot.black())
                .map(|user| user.username.as_str())
                .unwrap_or("Black");
            let identity =
                slot_card_label(i18n, tournament, &slot).unwrap_or_else(|| String::from("Game"));
            Some(ReviewedCloseoutSlot {
                id,
                order: slot_order(&slot),
                identity,
                players: format!("{white} vs {black}"),
                scheduled_at: slot.scheduled_at,
                released: slot.game.is_some(),
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
        TournamentFormatStore::RoundRobin(state) => state.closeout_eligible_slots().get(),
        TournamentFormatStore::Swiss(state) => state.closeout_eligible_slots().get(),
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
            <div class="p-3 space-y-2 ui-danger-notice" data-testid="tournament-closeout">
                <div>
                    <p class="font-bold">{t!(i18n, tournaments.view.closeout.title)}</p>
                    <p class="text-sm">
                        {move || {
                            t_string!(
                                i18n, tournaments.view.closeout.description, count = closeout_eligible_slots.get()
                            )
                        }}
                    </p>
                </div>
                <button
                    type="button"
                    class="ui-button ui-button-danger ui-button-sm"
                    on:click=move |_| {
                        if !route_active.get_untracked() {
                            return;
                        }
                        let rows = reviewed_closeout_slots(tournament, i18n);
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
                    {t!(i18n, tournaments.view.closeout.review)}
                </button>
            </div>
        </Show>
        // TODO: i18n once copy is approved.
        <Modal dialog_el aria_label="Review double forfeits" on_close=cancel>
            <Show when=move || route_active.get() && reviewed.with(Option::is_some)>
                <div class="px-3 pb-4 mx-auto space-y-3 sm:px-4 w-[min(94vw,48rem)]">
                    <header>
                        // TODO: i18n once copy is approved.
                        <h2 class="text-xl font-bold">
                            {move || {
                                format!(
                                    "Review {} double forfeits",
                                    reviewed.with(|rows| rows.as_ref().map_or(0, Vec::len)),
                                )
                            }}
                        </h2>
                        <p class="text-sm text-gray-600 dark:text-gray-300">
                            {move || tournament.common.lifecycle().get().name}
                        </p>
                    </header>
                    // TODO: i18n once copy is approved.
                    <p class="p-3 text-sm ui-danger-notice">
                        "Both players receive 0 points. This may finish the tournament."
                    </p>
                    // TODO: i18n once copy is approved.
                    <p class="text-xs text-gray-600 dark:text-gray-300">
                        {format!("Your local time · currently {}", Local::now().format("UTC%:z"))}
                    </p>
                    <div class="overflow-hidden rounded border divide-y border-black/10 divide-black/10 dark:border-white/10 dark:divide-white/10">
                        // TODO: i18n once copy is approved.
                        <div class="hidden gap-3 p-3 text-xs font-semibold sm:grid sm:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_minmax(0,1.3fr)_4.5rem]">
                            <span>"Game"</span>
                            <span>"Players"</span>
                            <span>"Agreed time"</span>
                            <span>"State"</span>
                        </div>
                        <For
                            each=move || {
                                reviewed
                                    .with(|rows| {
                                        rows.as_ref()
                                            .map(|rows| {
                                                rows.iter()
                                                    .skip((page.get() - 1) * 6)
                                                    .take(6)
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
                                class="grid gap-1 p-3 text-sm sm:gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_minmax(0,1.3fr)_4.5rem]"
                                data-slot-id=row.id.to_string()
                            >
                                <p>{row.identity}</p>
                                <p class="font-semibold break-words">{row.players}</p>
                                // TODO: i18n once copy is approved.
                                <p class="text-gray-600 dark:text-gray-300">
                                    {row
                                        .scheduled_at
                                        .map(|time| format_local_datetime(i18n.get_locale(), time))
                                        .unwrap_or_else(|| String::from("No agreed time"))}
                                </p>
                                // TODO: i18n once copy is approved.
                                <p>{if row.released { "Released" } else { "Planned" }}</p>
                            </article>
                        </For>
                    </div>
                    <PaginationControls
                        page=page.into()
                        total=Signal::derive(move || {
                            reviewed.with(|rows| rows.as_ref().map_or(0, Vec::len))
                        })
                        page_size=6
                        on_page_change=Callback::new(move |next| page.set(next))
                    />
                    <footer class="flex flex-wrap gap-2 justify-end pt-3 border-t border-black/10 dark:border-white/10">
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-md"
                            on:click=move |_| cancel.run(())
                        >
                            {t!(i18n, tournaments.detail.cancel)}
                        </button>
                        // TODO: i18n once copy is approved.
                        <button
                            type="button"
                            class="ui-button ui-button-danger ui-button-md"
                            on:click=move |_| confirm.run(())
                        >
                            {move || {
                                format!(
                                    "Record {} double forfeits",
                                    reviewed.with(|rows| rows.as_ref().map_or(0, Vec::len)),
                                )
                            }}
                        </button>
                    </footer>
                </div>
            </Show>
        </Modal>
    }
}
