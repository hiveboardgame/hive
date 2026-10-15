use crate::{
    common::{
        format_local_datetime as format_schedule_time,
        tournament_slot_opponent,
        ScheduleAction,
    },
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

fn offer_status_label(status: ScheduleOfferStatus) -> String {
    let i18n = use_i18n();
    match status {
        ScheduleOfferStatus::Pending => {
            t_string!(i18n, tournaments.scheduling.waiting_for_a_response).to_string()
        }
        ScheduleOfferStatus::Accepted => t_string!(i18n, common.accepted).to_string(),
        ScheduleOfferStatus::Declined => t_string!(i18n, common.declined).to_string(),
        ScheduleOfferStatus::Withdrawn => t_string!(i18n, common.withdrawn).to_string(),
        ScheduleOfferStatus::Superseded => {
            t_string!(i18n, tournaments.scheduling.replaced_by_newer_times).to_string()
        }
        ScheduleOfferStatus::Cancelled => {
            t_string!(i18n, tournaments.scheduling.closed).to_string()
        }
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
    can_propose: Signal<bool>,
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
        let accept_error = RwSignal::new(false);
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
                    <h3 class="font-bold">
                        {t!(i18n, tournaments.scheduling.select_a_proposed_time)}
                    </h3>
                    <p class="text-sm">
                        {move || {
                            t_string!(
                                i18n, tournaments.scheduling.value1_suggested_these_times_in_your_timezone, value1 = (offer.proposer_username.clone())
                            )
                                .to_string()
                        }}
                    </p>
                </div>
                <Show
                    when=move || !candidates.get().is_empty()
                    fallback=move || {
                        view! {
                            <p class="text-sm">
                                {move || {
                                    if can_propose.get() {
                                        t_string!(
                                            i18n, tournaments.scheduling.expired_times_suggest
                                        )
                                            .to_string()
                                    } else {
                                        t_string!(
                                            i18n, tournaments.scheduling.expired_times_decline
                                        )
                                            .to_string()
                                    }
                                }}
                            </p>
                        }
                    }
                >
                    <fieldset class="grid gap-2">
                        <legend class="sr-only">
                            {t!(i18n, tournaments.scheduling.choose_one_proposed_time)}
                        </legend>
                        <For each=move || candidates.get() key=|candidate| *candidate let:candidate>
                            <label class="flex gap-3 items-center p-3 cursor-pointer ui-card-row">
                                <input
                                    type="radio"
                                    name=format!("schedule-offer-{offer_id}", offer_id = (offer_id))
                                        .to_string()
                                    prop:checked=move || selected_candidate.get() == Some(candidate)
                                    prop:disabled=move || acceptance_sent.get().is_some()
                                    on:change=move |_| {
                                        selected_candidate.set(Some(candidate));
                                        accept_error.set(false);
                                    }
                                />
                                <time class="font-semibold" datetime=candidate.to_rfc3339()>
                                    {move || format_schedule_time(i18n.get_locale(), candidate)}
                                </time>
                            </label>
                        </For>
                    </fieldset>
                    <Show when=move || accept_error.get()>
                        <p class="ui-field-error">
                            {t!(
                                i18n, tournaments.scheduling.that_time_has_passed_choose_another_proposed_time
                            )}
                        </p>
                    </Show>
                    <button
                        type="button"
                        class="w-full ui-button ui-button-primary ui-button-md"
                        prop:disabled=move || {
                            !can_propose.get() || selected_candidate.get().is_none()
                                || acceptance_sent.get().is_some()
                        }
                        on:click=move |_| {
                            if route_active.try_get() != Some(true) || !can_propose.get_untracked()
                            {
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
                                accept_error.set(true);
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
                        {move || {
                            if acceptance_sent.get().is_some() {
                                t_string!(i18n, tournaments.scheduling.confirming_selected_time)
                                    .to_string()
                            } else {
                                selected_candidate
                                    .get()
                                    .map(|time| {
                                        t_string!(
                                            i18n, tournaments.scheduling.confirm_value1, value1 = (format_schedule_time(i18n.get_locale(), time))
                                        )
                                            .to_string()
                                    })
                                    .unwrap_or_else(|| {
                                        t_string!(i18n, tournaments.scheduling.choose_a_time)
                                            .to_string()
                                    })
                            }
                        }}
                    </button>
                </Show>
                <Show when=move || can_propose.get()>
                    <button
                        type="button"
                        class="w-full ui-button ui-button-secondary ui-button-md"
                        prop:disabled=move || acceptance_sent.get().is_some()
                        on:click=move |_| show_proposal_form.set(true)
                    >
                        {move || {
                            if candidates.with(Vec::is_empty) {
                                t_string!(i18n, tournaments.scheduling.suggest_new_times)
                                    .to_string()
                            } else {
                                t_string!(i18n, tournaments.scheduling.suggest_other_times_instead)
                                    .to_string()
                            }
                        }}
                    </button>
                    <p class="text-xs text-gray-600 dark:text-gray-300">
                        {t!(i18n, tournaments.scheduling.counterproposal_help)}
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
                    {t!(i18n, tournaments.scheduling.none_of_these_work)}
                </button>
            </section>
        }
        .into_any()
    } else {
        view! {
            <section class="p-3 space-y-2 ui-notice">
                <div>
                    <h3 class="font-bold">
                        {move || {
                            if candidates.with(Vec::is_empty) {
                                t_string!(i18n, tournaments.scheduling.proposed_times_expired)
                                    .to_string()
                            } else {
                                t_string!(i18n, tournaments.scheduling.waiting_for_your_opponent)
                                    .to_string()
                            }
                        }}
                    </h3>
                    <p class="text-sm">
                        {move || {
                            t_string!(
                                i18n, tournaments.scheduling.value1_can_accept_one_of_these_times, value1 = (offer.opponent_username.clone())
                            )
                                .to_string()
                        }}
                    </p>
                </div>
                <Show
                    when=move || !candidates.get().is_empty()
                    fallback=move || {
                        view! {
                            <p class="text-sm">
                                {move || {
                                    if can_propose.get() {
                                        t_string!(
                                            i18n, tournaments.scheduling.expired_times_replace
                                        )
                                            .to_string()
                                    } else {
                                        t_string!(
                                            i18n, tournaments.scheduling.expired_times_withdraw
                                        )
                                            .to_string()
                                    }
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
                    <Show when=move || can_propose.get()>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-sm"
                            on:click=move |_| show_proposal_form.set(true)
                        >
                            {move || {
                                if candidates.with(Vec::is_empty) {
                                    t_string!(i18n, tournaments.scheduling.suggest_new_times)
                                        .to_string()
                                } else {
                                    t_string!(i18n, tournaments.scheduling.replace_proposed_times)
                                        .to_string()
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
                        {t!(i18n, tournaments.scheduling.withdraw_proposal)}
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
                        <span class="font-semibold">{t!(i18n, common.selected)}</span>
                    </Show>
                </li>
            </For>
        </ul>
    }
}

#[component]
fn OfferHistory(offers: Memo<Vec<ScheduleResponse>>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <Show when=move || !offers.with(Vec::is_empty)>
            <details class="pt-2 border-t border-gray-200 dark:border-gray-700">
                <summary class="text-sm font-semibold cursor-pointer">
                    {t!(i18n, tournaments.scheduling.scheduling_history)}
                </summary>
                <ol class="mt-3 space-y-2 text-sm">
                    <For each=move || offers.get() key=|offer| offer.id let:offer>
                        <li class="p-2 ui-card-row">
                            <div class="flex flex-wrap gap-2 justify-between">
                                <span>{offer.proposer_username}</span>
                                <span class="text-xs font-semibold">
                                    {move || offer_status_label(offer.status)}
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

#[derive(Clone, Copy)]
enum ProposalInputError {
    Incomplete,
    NotFuture,
}

#[component]
fn ScheduleEditorBody(
    tournament_id: TournamentId,
    slot: Signal<SlotResponse>,
    opponent_name: Signal<String>,
    tournament_name: Signal<String>,
    slot_label: Signal<String>,
    current_user: Uuid,
    can_propose: Signal<bool>,
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
    let input_error = RwSignal::new(Option::<ProposalInputError>::None);
    let slot_id = slot.get_untracked().id;
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
    let current_time = Signal::derive(move || slot.with(|slot| slot.scheduled_at));
    let has_initial_pending = schedules.with_untracked(|all| {
        all.values().any(|offer| {
            offer.tournament_id == stored_tournament_id.get_value()
                && offer.slot_id == slot_id
                && offer.is_pending()
        })
    });
    let show_proposal_form =
        RwSignal::new(current_time.get_untracked().is_none() && !has_initial_pending);
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
        if route_active.try_get() != Some(true) || !can_propose.get_untracked() {
            return;
        }
        if acceptance_sent.get_untracked().is_some() {
            return;
        }
        if !drafts_valid.get_untracked() {
            input_error.set(Some(ProposalInputError::Incomplete));
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
            input_error.set(Some(ProposalInputError::NotFuture));
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
    let deadline = Signal::derive(move || slot.with(|slot| slot.deadline_at));
    let min = Local::now() + Duration::minutes(10);
    let max = min + Duration::weeks(12);

    view! {
        <div class="px-3 pb-4 mx-auto space-y-3 sm:px-4 w-[min(94vw,34rem)]">
            <div>
                <p class="text-xs font-semibold text-gray-500 break-words">{tournament_name}</p>
                <h2 class="text-xl font-bold break-words">
                    {move || {
                        t_string!(
                            i18n, tournaments.scheduling.schedule_with_value1, value1 = (opponent_name.get())
                        )
                            .to_string()
                    }}
                </h2>
                <p class="text-sm">{slot_label}</p>
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    {move || {
                        t_string!(
                            i18n, tournaments.ui.your_local_time_currently, value1 = (Local::now().format("UTC%:z"))
                        )
                            .to_string()
                    }}
                </p>
            </div>

            <Show when=move || current_time.get().is_some()>
                <section class="p-3 ui-success-notice">
                    <p class="text-xs font-bold">{t!(i18n, tournaments.ui.agreed_time)}</p>
                    {move || {
                        current_time
                            .get()
                            .map(|time| {
                                view! {
                                    <time class="font-bold" datetime=time.to_rfc3339()>
                                        {move || format_schedule_time(i18n.get_locale(), time)}
                                    </time>
                                }
                            })
                    }}
                </section>
            </Show>

            <Show when=move || deadline.get().is_some()>
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    <span class="font-semibold">{t!(i18n, tournaments.scheduling.play_by)}</span>
                    {move || {
                        deadline.get().map(|time| format_schedule_time(i18n.get_locale(), time))
                    }}
                    <span>{t!(i18n, tournaments.scheduling.no_automatic_result_consequence)}</span>
                </p>
            </Show>

            <For
                each=move || { pending_offer().into_iter().collect::<Vec<_>>() }
                key=|offer| offer.id
                children=move |offer| {
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
                }
            />

            <Show when=move || {
                acceptance_sent
                    .get()
                    .is_some_and(|selected_time| Some(selected_time) != current_time.get())
                    && pending_offer().is_none()
            }>
                <p class="text-sm ui-notice">
                    {t!(i18n, tournaments.scheduling.saving_the_agreed_time)}
                </p>
            </Show>

            <Show when=move || {
                current_time.get().is_some() && pending_offer().is_none()
                    && !show_proposal_form.get()
                    && acceptance_sent
                        .get()
                        .is_none_or(|selected_time| Some(selected_time) == current_time.get())
            }>
                <div class="flex flex-wrap gap-2">
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-md"
                        on:click=move |_| {
                            let _ = close_editor.try_run(());
                        }
                    >
                        {t!(i18n, common.done)}
                    </button>
                    <Show when=move || can_propose.get()>
                        <button
                            type="button"
                            class="ui-button ui-button-danger ui-button-md"
                            on:click=move |_| {
                                if route_active.try_get() == Some(true)
                                    && can_propose.get_untracked()
                                {
                                    if let Some(expected_time) = current_time.get_untracked() {
                                        api.get()
                                            .schedule_action(ScheduleAction::CancelAgreement {
                                                tournament_id: stored_tournament_id.get_value(),
                                                slot_id,
                                                expected_time,
                                            });
                                        let _ = close_editor.try_run(());
                                    }
                                }
                            }
                        >
                            {t!(i18n, tournaments.scheduling.cancel_agreed_time)}
                        </button>
                        <button
                            type="button"
                            class="ui-button ui-button-secondary ui-button-md"
                            on:click=move |_| show_proposal_form.set(true)
                        >
                            {t!(i18n, tournaments.scheduling.propose_a_different_time)}
                        </button>
                    </Show>
                </div>
            </Show>

            <Show
                when=move || can_propose.get()
                fallback=move || {
                    view! {
                        <p class="text-sm ui-notice">
                            {t!(i18n, tournaments.scheduling.rescheduling_closed)}
                        </p>
                    }
                }
            >
                <Show when=move || show_proposal_form.get() && acceptance_sent.get().is_none()>
                    <section class="space-y-3">
                        <div>
                            <h3 class="font-bold">
                                {move || {
                                    if pending_offer().is_some() {
                                        t_string!(i18n, tournaments.scheduling.suggest_other_times)
                                            .to_string()
                                    } else if current_time.get().is_some() {
                                        t_string!(i18n, tournaments.scheduling.change_the_time)
                                            .to_string()
                                    } else {
                                        t_string!(i18n, tournaments.scheduling.propose_a_time)
                                            .to_string()
                                    }
                                }}
                            </h3>
                            <p class="text-sm text-gray-600 dark:text-gray-300">
                                {t!(
                                    i18n, tournaments.scheduling.offer_up_to_three_alternatives_at_once
                                )}
                            </p>
                        </div>
                        <div class="grid gap-3 sm:grid-cols-2">
                            <For
                                each=move || { (0..candidate_count.get()).collect::<Vec<_>>() }
                                key=|index| *index
                                let:index
                            >
                                <DateTimePicker
                                    input_id=format!(
                                        "schedule-candidate-{slot_id}-{index}",
                                        slot_id = (slot_id),
                                        index = (index),
                                    )
                                        .to_string()
                                    text=move || {
                                        t_string!(
                                            i18n, tournaments.scheduling.option_value1, value1 = (index + 1)
                                        )
                                            .to_string()
                                    }
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
                                    {t!(i18n, tournaments.scheduling.add_another_time)}
                                </button>
                            </Show>
                            <Show when=move || { candidate_count.get() > 1 }>
                                <button
                                    type="button"
                                    class="ui-button ui-button-ghost ui-button-sm"
                                    on:click=move |_| candidate_count.update(|count| *count -= 1)
                                >
                                    {t!(i18n, tournaments.scheduling.remove_last)}
                                </button>
                            </Show>
                        </div>
                        <ShowLet some=move || input_error.get() let:error>
                            <p class="ui-field-error">
                                {move || match error {
                                    ProposalInputError::Incomplete => {
                                        t_string!(
                                            i18n, tournaments.scheduling.complete_each_proposed_date_and_time_before_sending
                                        )
                                            .to_string()
                                    }
                                    ProposalInputError::NotFuture => {
                                        t_string!(
                                            i18n, tournaments.scheduling.choose_future_times_before_sending
                                        )
                                            .to_string()
                                    }
                                }}
                            </p>
                        </ShowLet>
                        <button
                            type="button"
                            class="w-full ui-button ui-button-primary ui-button-md"
                            prop:disabled=move || !drafts_valid.get()
                            on:click=move |_| {
                                let _ = send.try_run(());
                            }
                        >
                            {t!(i18n, tournaments.scheduling.send_proposed_times)}
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
        <Modal
            dialog_el
            aria_label=move || t_string!(i18n, tournaments.scheduling.schedule_match).to_string()
            on_close=close_editor
        >
            <For
                each=move || {
                    if route_active.try_get() != Some(true) {
                        return Vec::new();
                    }
                    selected_slot
                        .try_get()
                        .flatten()
                        .zip(user_id.try_get().flatten())
                        .filter(|(slot_id, _)| {
                            tournament
                                .format
                                .slot(*slot_id)
                                .and_then(|slot| slot.try_get())
                                .is_some()
                        })
                        .map(|(slot_id, current_user)| (
                            tournament.tournament_id(),
                            slot_id,
                            current_user,
                        ))
                        .into_iter()
                        .collect::<Vec<_>>()
                }
                key=|identity| identity.clone()
                children=move |(tournament_id, slot_id, current_user)| {
                    let slot_field = tournament
                        .format
                        .slot(slot_id)
                        .expect("selected schedule slot exists");
                    let slot = Signal::derive(move || slot_field.get());
                    let can_propose = Signal::derive(move || {
                        route_active.get() && schedule_is_available(&slot.get(), current_user)
                            && tournament
                                .common
                                .memberships()
                                .with(|memberships| {
                                    memberships.players.contains_key(&current_user)
                                        && !memberships.withdrawn.contains(&current_user)
                                })
                    });
                    let opponent_name = Signal::derive(move || {
                        let opponent = tournament_slot_opponent(&slot.get(), current_user);
                        tournament
                            .common
                            .memberships()
                            .with(|memberships| {
                                opponent
                                    .and_then(|id| memberships.players.get(&id))
                                    .map(|player| player.username.clone())
                                    .unwrap_or_else(|| {
                                        t_string!(i18n, tournaments.scheduling.your_opponent)
                                            .to_string()
                                    })
                            })
                    });
                    let tournament_name = Signal::derive(move || {
                        tournament.common.lifecycle().get().name
                    });
                    let slot_label = Signal::derive(move || {
                        slot_card_label(i18n, tournament, &slot.get())
                            .unwrap_or_else(|| t_string!(i18n, common.not_started).to_string())
                    });
                    ScheduleEditorBody(ScheduleEditorBodyProps {
                        tournament_id,
                        slot,
                        opponent_name,
                        tournament_name,
                        slot_label,
                        current_user,
                        can_propose,
                        schedules,
                        close_editor,
                        route_active,
                    })
                }
            />
        </Modal>
    }
}
