use crate::{
    common::{format_local_datetime as format_schedule_time, ScheduleAction},
    components::{
        atoms::date_time_picker::DateTimePicker,
        molecules::modal::Modal,
        organisms::tournament_slots::{schedule_is_available, slot_card_label},
    },
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{
        ApiRequestsProvider,
        TournamentCommonStoreFields,
        TournamentScheduleState,
        TournamentState,
    },
    responses::ScheduleResponse,
};
use chrono::{DateTime, Duration, Local, Timelike, Utc};
use leptos::{html::Dialog, prelude::*};
use shared_types::{tournament_view::SlotResponse, ScheduleOfferStatus, TournamentId};
use std::cmp::Reverse;
use uuid::Uuid;

fn offer_status_label(status: ScheduleOfferStatus) -> &'static str {
    // TODO: i18n once copy is approved.
    match status {
        ScheduleOfferStatus::Pending => "Waiting for a response",
        ScheduleOfferStatus::Accepted => "Accepted",
        ScheduleOfferStatus::Declined => "Declined",
        ScheduleOfferStatus::Withdrawn => "Withdrawn",
        ScheduleOfferStatus::Superseded => "Replaced by newer times",
        ScheduleOfferStatus::Cancelled => "Closed",
    }
}

fn initial_candidate_time() -> DateTime<Utc> {
    (Utc::now() + Duration::hours(1))
        .with_second(0)
        .and_then(|time| time.with_nanosecond(0))
        .unwrap_or_else(|| Utc::now() + Duration::hours(1))
}

#[component]
fn PendingOfferPanel(
    offer: ScheduleResponse,
    current_user: Uuid,
    can_propose: bool,
    show_proposal_form: RwSignal<bool>,
    acceptance_sent: RwSignal<Option<DateTime<Utc>>>,
    close_editor: Callback<()>,
    route_active: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let offer_id = offer.id;
    let incoming = offer.opponent_id == current_user;
    let offered_candidates = StoredValue::new(offer.candidate_times.clone());
    let now = use_ticking_now();
    let candidates = Memo::new(move |_| {
        offered_candidates.with_value(|candidates| {
            candidates
                .iter()
                .copied()
                .filter(|candidate| *candidate > now.get())
                .collect::<Vec<_>>()
        })
    });

    if incoming {
        let selected_candidate = RwSignal::new(candidates.get().first().copied());
        let accept_error = RwSignal::new(Option::<String>::None);
        Effect::new(move |_| {
            let available = candidates.get();
            if selected_candidate
                .get_untracked()
                .is_some_and(|selected| !available.contains(&selected))
            {
                selected_candidate.set(available.first().copied());
            }
        });
        view! {
            <section class="p-3 space-y-3 ui-warning-notice">
                <div>
                    // TODO: i18n once copy is approved.
                    <h3 class="font-bold">"Select a proposed time"</h3>
                    // TODO: i18n once copy is approved.
                    <p class="text-sm">
                        {format!(
                            "{} suggested these times in your timezone.",
                            offer.proposer_username,
                        )}
                    </p>
                </div>
                <Show
                    when=move || !candidates.get().is_empty()
                    fallback=move || {
                        view! {
                            // TODO: i18n once copy is approved.
                            <p class="text-sm">
                                {if can_propose {
                                    "These proposed times have passed. Suggest new times instead."
                                } else {
                                    "These proposed times have passed. Decline the proposal."
                                }}
                            </p>
                        }
                    }
                >
                    <fieldset class="grid gap-2">
                        // TODO: i18n once copy is approved.
                        <legend class="sr-only">"Choose one proposed time"</legend>
                        <For each=move || candidates.get() key=|candidate| *candidate let:candidate>
                            <label class="flex gap-3 items-center p-3 cursor-pointer ui-card-row">
                                <input
                                    type="radio"
                                    name=format!("schedule-offer-{offer_id}")
                                    prop:checked=move || selected_candidate.get() == Some(candidate)
                                    prop:disabled=move || acceptance_sent.get().is_some()
                                    on:change=move |_| {
                                        selected_candidate.set(Some(candidate));
                                        accept_error.set(None);
                                    }
                                />
                                <time class="font-semibold" datetime=candidate.to_rfc3339()>
                                    {move || format_schedule_time(i18n.get_locale(), candidate)}
                                </time>
                            </label>
                        </For>
                    </fieldset>
                    <ShowLet some=move || accept_error.get() let:error>
                        <p class="ui-field-error">{error}</p>
                    </ShowLet>
                    <button
                        type="button"
                        class="w-full ui-button ui-button-primary ui-button-md"
                        prop:disabled=move || {
                            selected_candidate.get().is_none() || acceptance_sent.get().is_some()
                        }
                        on:click=move |_| {
                            if route_active.try_get() != Some(true) {
                                return;
                            }
                            if acceptance_sent.get_untracked().is_some() {
                                return;
                            }
                            let Some(selected_time) = selected_candidate.get_untracked() else {
                                return;
                            };
                            if selected_time <= Utc::now() {
                                selected_candidate.set(None);
                                accept_error
                                    .set(
                                        Some(
                                            String::from(
                                                "That time has passed. Choose another proposed time.",
                                            ),
                                        ),
                                    );
                                return;
                            }
                            show_proposal_form.set(false);
                            acceptance_sent.set(Some(selected_time));
                            api.get()
                                .schedule_action(ScheduleAction::Accept {
                                    offer_id,
                                    selected_time,
                                });
                        }
                    >
                        // TODO: i18n once copy is approved.
                        {move || {
                            if acceptance_sent.get().is_some() {
                                String::from("Confirming selected time…")
                            } else {
                                selected_candidate
                                    .get()
                                    .map(|time| {
                                        format!(
                                            "Confirm {}",
                                            format_schedule_time(i18n.get_locale(), time),
                                        )
                                    })
                                    .unwrap_or_else(|| String::from("Choose a time"))
                            }
                        }}
                    </button>
                </Show>
                <Show when=move || can_propose>
                    <button
                        type="button"
                        class="w-full ui-button ui-button-secondary ui-button-md"
                        prop:disabled=move || acceptance_sent.get().is_some()
                        on:click=move |_| show_proposal_form.set(true)
                    >
                        // TODO: i18n once copy is approved.
                        {move || {
                            if candidates.with(Vec::is_empty) {
                                "Suggest new times"
                            } else {
                                "Suggest other times instead"
                            }
                        }}
                    </button>
                    // TODO: i18n once copy is approved.
                    <p class="text-xs text-gray-600 dark:text-gray-300">
                        "This sends a counterproposal; it does not accept one of the times above."
                    </p>
                </Show>
                <button
                    type="button"
                    class="ui-button ui-button-ghost ui-button-sm"
                    prop:disabled=move || acceptance_sent.get().is_some()
                    on:click=move |_| {
                        if route_active.try_get() != Some(true) {
                            return;
                        }
                        api.get().schedule_action(ScheduleAction::Decline(offer_id));
                        let _ = close_editor.try_run(());
                    }
                >
                    // TODO: i18n once copy is approved.
                    "None of these work"
                </button>
            </section>
        }
        .into_any()
    } else {
        view! {
            <section class="p-3 space-y-2 ui-notice">
                <div>
                    // TODO: i18n once copy is approved.
                    <h3 class="font-bold">
                        {move || {
                            if candidates.with(Vec::is_empty) {
                                "Proposed times expired"
                            } else {
                                "Waiting for your opponent"
                            }
                        }}
                    </h3>
                    // TODO: i18n once copy is approved.
                    <p class="text-sm">
                        {format!("{} can accept one of these times.", offer.opponent_username)}
                    </p>
                </div>
                <Show
                    when=move || !candidates.get().is_empty()
                    fallback=move || {
                        view! {
                            // TODO: i18n once copy is approved.
                            <p class="text-sm">
                                {if can_propose {
                                    "All of your proposed times have passed. Replace or withdraw this proposal."
                                } else {
                                    "All of your proposed times have passed. Withdraw this proposal."
                                }}
                            </p>
                        }
                    }
                >
                    <ul class="space-y-1 text-sm">
                        <For each=move || candidates.get() key=|candidate| *candidate let:candidate>
                            <li>
                                <time datetime=candidate
                                    .to_rfc3339()>
                                    {move || format_schedule_time(i18n.get_locale(), candidate)}
                                </time>
                            </li>
                        </For>
                    </ul>
                </Show>
                <div class="flex flex-wrap gap-2">
                    <Show when=move || can_propose>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-sm"
                            on:click=move |_| show_proposal_form.set(true)
                        >
                            // TODO: i18n once copy is approved.
                            {move || {
                                if candidates.with(Vec::is_empty) {
                                    "Suggest new times"
                                } else {
                                    "Replace proposed times"
                                }
                            }}
                        </button>
                    </Show>
                    <button
                        type="button"
                        class="ui-button ui-button-ghost ui-button-sm"
                        on:click=move |_| {
                            if route_active.try_get() != Some(true) {
                                return;
                            }
                            api.get().schedule_action(ScheduleAction::Withdraw(offer_id));
                            let _ = close_editor.try_run(());
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Withdraw proposal"
                    </button>
                </div>
            </section>
        }
        .into_any()
    }
}

#[component]
fn OfferHistoryCandidates(
    candidate_times: Vec<DateTime<Utc>>,
    selected_time: Option<DateTime<Utc>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let candidate_times = StoredValue::new(candidate_times);
    view! {
        <ul class="mt-1 space-y-1 text-xs text-gray-600 dark:text-gray-300">
            <For each=move || candidate_times.get_value() key=|candidate| *candidate let:candidate>
                <li class="flex flex-wrap gap-2 justify-between">
                    <time datetime=candidate
                        .to_rfc3339()>
                        {move || format_schedule_time(i18n.get_locale(), candidate)}
                    </time>
                    <Show when=move || selected_time == Some(candidate)>
                        // TODO: i18n once copy is approved.
                        <span class="font-semibold">"Selected"</span>
                    </Show>
                </li>
            </For>
        </ul>
    }
}

#[component]
fn OfferHistory(offers: Memo<Vec<ScheduleResponse>>) -> impl IntoView {
    view! {
        <Show when=move || !offers.with(Vec::is_empty)>
            <details class="pt-2 border-t border-gray-200 dark:border-gray-700">
                // TODO: i18n once copy is approved.
                <summary class="text-sm font-semibold cursor-pointer">"Scheduling history"</summary>
                <ol class="mt-3 space-y-2 text-sm">
                    <For each=move || offers.get() key=|offer| offer.id let:offer>
                        <li class="p-2 ui-card-row">
                            <div class="flex flex-wrap gap-2 justify-between">
                                <span>{offer.proposer_username}</span>
                                <span class="text-xs font-semibold">
                                    {offer_status_label(offer.status)}
                                </span>
                            </div>
                            <OfferHistoryCandidates
                                candidate_times=offer.candidate_times
                                selected_time=offer.selected_time
                            />
                        </li>
                    </For>
                </ol>
            </details>
        </Show>
    }
}

#[component]
fn ScheduleEditorBody(
    tournament_id: TournamentId,
    slot: SlotResponse,
    opponent_name: String,
    tournament_name: String,
    slot_label: String,
    current_user: Uuid,
    can_propose: bool,
    schedules: TournamentScheduleState,
    close_editor: Callback<()>,
    route_active: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let base = initial_candidate_time();
    let candidate_times = [
        RwSignal::new(base),
        RwSignal::new(base + Duration::days(1)),
        RwSignal::new(base + Duration::days(2)),
    ];
    let candidate_count = RwSignal::new(1usize);
    let candidate_valid = [
        RwSignal::new(true),
        RwSignal::new(true),
        RwSignal::new(true),
    ];
    let drafts_valid = Signal::derive(move || {
        candidate_valid
            .iter()
            .take(candidate_count.get())
            .all(|valid| valid.get())
    });
    let input_error = RwSignal::new(Option::<String>::None);
    let slot_id = slot.id;
    let stored_tournament_id = StoredValue::new(tournament_id.clone());
    let pending_offer = move || {
        schedules.with(|all| {
            all.values()
                .find(|offer| {
                    offer.tournament_id == stored_tournament_id.get_value()
                        && offer.slot_id == slot_id
                        && offer.is_pending()
                })
                .cloned()
        })
    };
    let current_time = slot.scheduled_at;
    let has_initial_pending = schedules.with_untracked(|all| {
        all.values().any(|offer| {
            offer.tournament_id == stored_tournament_id.get_value()
                && offer.slot_id == slot_id
                && offer.is_pending()
        })
    });
    let show_proposal_form = RwSignal::new(current_time.is_none() && !has_initial_pending);
    let acceptance_sent = RwSignal::new(Option::<DateTime<Utc>>::None);
    let history = Memo::new(move |_| {
        let mut history = schedules.with(|all| {
            all.values()
                .filter(|offer| {
                    offer.tournament_id == stored_tournament_id.get_value()
                        && offer.slot_id == slot_id
                        && !offer.is_pending()
                })
                .cloned()
                .collect::<Vec<_>>()
        });
        history.sort_by_key(|offer| Reverse((offer.created_at, offer.id)));
        history
    });
    let send = Callback::new(move |_: ()| {
        if route_active.try_get() != Some(true) {
            return;
        }
        if acceptance_sent.get_untracked().is_some() {
            return;
        }
        if !drafts_valid.get_untracked() {
            // TODO: i18n once copy is approved.
            input_error.set(Some(String::from(
                "Complete each proposed date and time before sending.",
            )));
            return;
        }
        let mut candidates = candidate_times
            .iter()
            .take(candidate_count.get_untracked())
            .map(|candidate| candidate.get_untracked())
            .collect::<Vec<_>>();
        candidates.sort();
        candidates.dedup();
        if candidates.is_empty() || candidates.iter().any(|candidate| *candidate <= Utc::now()) {
            // TODO: i18n once copy is approved.
            input_error.set(Some(String::from("Choose future times before sending.")));
            return;
        }
        input_error.set(None);
        api.get().schedule_action(ScheduleAction::Propose {
            candidate_times: candidates,
            tournament_id: tournament_id.clone(),
            slot_id,
        });
        let _ = close_editor.try_run(());
    });
    let deadline = slot.deadline_at;
    let min = Local::now() + Duration::minutes(10);
    let max = min + Duration::weeks(12);

    view! {
        <div class="px-3 pb-4 mx-auto space-y-3 sm:px-4 w-[min(94vw,34rem)]">
            <div>
                // TODO: i18n once copy is approved.
                <p class="text-xs font-semibold text-gray-500 break-words">{tournament_name}</p>
                // TODO: i18n once copy is approved.
                <h2 class="text-xl font-bold break-words">
                    {format!("Schedule with {opponent_name}")}
                </h2>
                <p class="text-sm">{slot_label}</p>
                // TODO: i18n once copy is approved.
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    {format!("Your local time · currently {}", Local::now().format("UTC%:z"))}
                </p>
            </div>

            <Show when=move || current_time.is_some()>
                <section class="p-3 ui-success-notice">
                    // TODO: i18n once copy is approved.
                    <p class="text-xs font-bold">"Agreed time"</p>
                    {current_time
                        .map(|time| {
                            view! {
                                <time class="font-bold" datetime=time.to_rfc3339()>
                                    {move || format_schedule_time(i18n.get_locale(), time)}
                                </time>
                            }
                        })}
                </section>
            </Show>

            <Show when=move || deadline.is_some()>
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    // TODO: i18n once copy is approved.
                    <span class="font-semibold">"Play by: "</span>
                    {move || deadline.map(|time| format_schedule_time(i18n.get_locale(), time))}
                    // TODO: i18n once copy is approved.
                    <span>" · no automatic result consequence"</span>
                </p>
            </Show>

            {move || {
                pending_offer()
                    .map(|offer| {
                        view! {
                            <PendingOfferPanel
                                offer
                                current_user
                                can_propose
                                show_proposal_form
                                acceptance_sent
                                close_editor
                                route_active
                            />
                        }
                    })
            }}

            <Show when=move || {
                acceptance_sent
                    .get()
                    .is_some_and(|selected_time| Some(selected_time) != current_time)
                    && pending_offer().is_none()
            }>
                <p class="text-sm ui-notice">
                    // TODO: i18n once copy is approved.
                    "Saving the agreed time…"
                </p>
            </Show>

            <Show when=move || {
                current_time.is_some() && pending_offer().is_none() && !show_proposal_form.get()
                    && acceptance_sent
                        .get()
                        .is_none_or(|selected_time| Some(selected_time) == current_time)
            }>
                <div class="flex flex-wrap gap-2">
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-md"
                        on:click=move |_| {
                            let _ = close_editor.try_run(());
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Done"
                    </button>
                    <Show when=move || can_propose>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-md"
                            on:click=move |_| show_proposal_form.set(true)
                        >
                            // TODO: i18n once copy is approved.
                            "Propose a different time"
                        </button>
                    </Show>
                </div>
            </Show>

            <Show
                when=move || can_propose
                fallback=move || {
                    view! {
                        <p class="text-sm ui-notice">
                            // TODO: i18n once copy is approved.
                            "This match can no longer be rescheduled. Its scheduling history remains available below."
                        </p>
                    }
                }
            >
                <Show when=move || show_proposal_form.get() && acceptance_sent.get().is_none()>
                    <section class="space-y-3">
                        <div>
                            // TODO: i18n once copy is approved.
                            <h3 class="font-bold">
                                {move || {
                                    if pending_offer().is_some() {
                                        "Suggest other times"
                                    } else if current_time.is_some() {
                                        "Change the time"
                                    } else {
                                        "Propose a time"
                                    }
                                }}
                            </h3>
                            // TODO: i18n once copy is approved.
                            <p class="text-sm text-gray-600 dark:text-gray-300">
                                "Offer up to three alternatives at once."
                            </p>
                        </div>
                        <div class="grid gap-3 sm:grid-cols-2">
                            <For
                                each=move || { (0..candidate_count.get()).collect::<Vec<_>>() }
                                key=|index| *index
                                let:index
                            >
                                <DateTimePicker
                                    input_id=format!("schedule-candidate-{slot_id}-{index}")
                                    // TODO: i18n once copy is approved.
                                    text=format!("Option {}", index + 1)
                                    min
                                    max
                                    draft_valid=candidate_valid[index]
                                    value=candidate_times[index]
                                        .get_untracked()
                                        .with_timezone(&Local)
                                    success_callback=Callback::from(move |time: DateTime<Utc>| {
                                        candidate_times[index].set(time);
                                        input_error.set(None);
                                    })
                                />
                            </For>
                        </div>
                        <div class="flex flex-wrap gap-2">
                            <Show when=move || { candidate_count.get() < 3 }>
                                <button
                                    type="button"
                                    class="ui-button ui-button-secondary ui-button-sm"
                                    on:click=move |_| candidate_count.update(|count| *count += 1)
                                >
                                    // TODO: i18n once copy is approved.
                                    "Add another time"
                                </button>
                            </Show>
                            <Show when=move || { candidate_count.get() > 1 }>
                                <button
                                    type="button"
                                    class="ui-button ui-button-ghost ui-button-sm"
                                    on:click=move |_| candidate_count.update(|count| *count -= 1)
                                >
                                    // TODO: i18n once copy is approved.
                                    "Remove last"
                                </button>
                            </Show>
                        </div>
                        <ShowLet some=move || input_error.get() let:error>
                            <p class="ui-field-error">{error}</p>
                        </ShowLet>
                        <button
                            type="button"
                            class="w-full ui-button ui-button-primary ui-button-md"
                            prop:disabled=move || !drafts_valid.get()
                            on:click=move |_| {
                                let _ = send.try_run(());
                            }
                        >
                            // TODO: i18n once copy is approved.
                            "Send proposed times"
                        </button>
                    </section>
                </Show>
            </Show>

            <OfferHistory offers=history />
        </div>
    }
}

#[component]
pub(crate) fn ScheduleEditorDialog(
    tournament: TournamentState,
    schedules: TournamentScheduleState,
    user_id: Signal<Option<Uuid>>,
    selected_slot: RwSignal<Option<Uuid>>,
    dialog_el: NodeRef<Dialog>,
    route_active: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let close_editor = Callback::new(move |_: ()| {
        if route_active.try_get() != Some(true) {
            return;
        }
        if let Some(dialog) = dialog_el.try_get().flatten() {
            dialog.close();
        }
        selected_slot.set(None);
    });
    view! {
        // TODO: i18n once copy is approved.
        <Modal dialog_el aria_label="Schedule match" on_close=close_editor>
            {move || {
                if !route_active.try_get()? {
                    return None;
                }
                let slot_id = selected_slot.try_get()??;
                let current_user = user_id.try_get()??;
                let memberships = tournament.common.memberships().try_get()?;
                let lifecycle = tournament.common.lifecycle().try_get()?;
                let editor = {
                    let slot = tournament.format.slot(slot_id)?.try_get()?;
                    let can_propose = schedule_is_available(&slot, current_user);
                    let opponent_id = if slot.white() == current_user {
                        slot.black()
                    } else {
                        slot.white()
                    };
                    let opponent_name = memberships
                        .players
                        .get(&opponent_id)
                        .map(|player| player.username.clone())
                        .unwrap_or_else(|| String::from("your opponent"));
                    let slot_label = slot_card_label(i18n, tournament, &slot)
                        .unwrap_or_else(|| String::from("Game 1"));
                    Some((
                        lifecycle.tournament_id,
                        slot,
                        opponent_name,
                        current_user,
                        can_propose,
                        lifecycle.name,
                        slot_label,
                    ))
                }?;
                Some(
                    ScheduleEditorBody(ScheduleEditorBodyProps {
                        tournament_id: editor.0,
                        slot: editor.1,
                        opponent_name: editor.2,
                        tournament_name: editor.5,
                        slot_label: editor.6,
                        current_user: editor.3,
                        can_propose: editor.4,
                        schedules,
                        close_editor,
                        route_active,
                    }),
                )
            }}
        </Modal>
    }
}
