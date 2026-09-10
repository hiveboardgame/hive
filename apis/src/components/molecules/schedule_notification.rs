use crate::{
    common::{format_local_datetime, schedule_slot_fragment, ScheduleAction},
    functions::schedules::MarkScheduleSeen,
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{ApiRequestsProvider, NotificationContext},
};
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;
use uuid::Uuid;

#[component]
pub fn ProposalNotification(
    tournament_name: String,
    slot_context: String,
    schedule_id: Uuid,
    proposer_username: String,
    tournament_id: TournamentId,
    slot_id: Uuid,
    candidate_times: Vec<DateTime<Utc>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let notifications = expect_context::<NotificationContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let schedule_id = StoredValue::new(schedule_id);
    let proposer_username = StoredValue::new(proposer_username);
    let candidate_times = StoredValue::new(candidate_times);
    let now = use_ticking_now();
    let valid_candidates = Memo::new(move |_| {
        candidate_times.with_value(|candidates| {
            candidates
                .iter()
                .copied()
                .filter(|candidate| *candidate > now.get())
                .collect::<Vec<_>>()
        })
    });
    let notification_text = move || {
        let candidates = valid_candidates.get();
        match candidates.as_slice() {
            [candidate] => t_string!(
                i18n,
                notifications.schedule.proposed,
                proposer = proposer_username.get_value(),
                time = format_local_datetime(i18n.get_locale(), *candidate),
            )
            .to_string(),
            [] => {
                // TODO: i18n once copy is approved.
                String::from("This scheduling proposal has expired")
            }
            candidates => {
                // TODO: i18n once copy is approved.
                format!(
                    "{} offered {} times",
                    proposer_username.get_value(),
                    candidates.len()
                )
            }
        }
    };
    let link_label = move || {
        if valid_candidates.with(|candidates| candidates.len() > 1) {
            // TODO: i18n once copy is approved.
            String::from("Review times")
        } else {
            t_string!(i18n, notifications.actions.view_tournament).to_string()
        }
    };

    let accept = move |_| {
        let candidates = valid_candidates.get();
        let [candidate] = candidates.as_slice() else {
            return;
        };
        let api = api.get();
        api.schedule_action(ScheduleAction::Accept {
            offer_id: schedule_id.get_value(),
            selected_time: *candidate,
        });
    };

    let decline = move |_| {
        let api = api.get();
        api.schedule_action(ScheduleAction::Decline(schedule_id.get_value()));
    };

    let dismiss = move |_| {
        notifications.schedule_notification_remove(schedule_id.get_value());
    };

    view! {
        <div class="flex-col gap-2 items-stretch ui-notification-item">
            <div class="flex gap-2 items-start w-full">
                <div class="ui-notification-item-body">
                    <div class="text-xs font-semibold break-words">
                        {tournament_name} " · " {slot_context}
                    </div>
                    <div class="whitespace-normal ui-notification-title">{notification_text}</div>
                    <div class="text-sm text-gray-600 dark:text-gray-400">
                        <a
                            href=format!(
                                "/tournament/{}/schedule/{}#{}",
                                &tournament_id.to_string(),
                                proposer_username.get_value(),
                                schedule_slot_fragment(slot_id),
                            )
                            class="ui-text-link"
                        >
                            {link_label}
                        </a>
                    </div>
                </div>
                <button
                    title=move || t_string!(i18n, notifications.actions.dismiss).to_string()
                    on:click=dismiss
                    class="z-20 ui-button ui-button-ghost ui-button-icon"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                </button>
            </div>
            <Show when=move || !valid_candidates.with(Vec::is_empty)>
                <div class="flex gap-2 justify-end">
                    <Show when=move || valid_candidates.with(|candidates| candidates.len() == 1)>
                        <button on:click=accept class="ui-button ui-button-success ui-button-sm">
                            {t!(i18n, notifications.actions.accept)}
                        </button>
                    </Show>
                    <button on:click=decline class="ui-button ui-button-danger ui-button-sm">
                        {move || {
                            if valid_candidates.with(|candidates| candidates.len() > 1) {
                                String::from("None work")
                            } else {
                                t_string!(i18n, notifications.actions.decline).to_string()
                            }
                        }}
                    </button>
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn AcceptanceNotification(
    tournament_name: String,
    schedule_id: Uuid,
    accepter_username: String,
    tournament_id: TournamentId,
    slot_id: Uuid,
    selected_time: DateTime<Utc>,
) -> impl IntoView {
    let i18n = use_i18n();
    let notifications = expect_context::<NotificationContext>();
    let schedule_id = StoredValue::new(schedule_id);
    let accepter_username = StoredValue::new(accepter_username);
    let notification_text = move || {
        t_string!(
            i18n,
            notifications.schedule.accepted,
            player = accepter_username.get_value(),
            time = format_local_datetime(i18n.get_locale(), selected_time),
        )
        .to_string()
    };

    let mark_seen_action = ServerAction::<MarkScheduleSeen>::new();

    let dismiss = move |_| {
        notifications.schedule_notification_remove(schedule_id.get_value());
    };

    view! {
        <div class="ui-notification-item">
            <div class="ui-notification-item-body">
                <div class="whitespace-normal ui-notification-title">{notification_text}</div>
                <div class="text-sm text-gray-600 dark:text-gray-400">
                    <a
                        href=format!(
                            "/tournament/{}/schedule/{}#tournament-slot-{slot_id}",
                            &tournament_id.to_string(),
                            accepter_username.get_value(),
                        )
                        class="ui-text-link"
                    >
                        {t!(
                            i18n,
                            notifications.actions.view_named_tournament,
                            name = tournament_name.clone(),
                        )}
                    </a>
                </div>
            </div>
            <ActionForm action=mark_seen_action on:submit=dismiss>
                <input type="hidden" name="schedule_id" value=schedule_id.get_value().to_string() />
                <button
                    type="submit"
                    title=move || t_string!(i18n, notifications.actions.dismiss).to_string()
                    class="z-50 ui-button ui-button-ghost ui-button-icon"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                </button>
            </ActionForm>
        </div>
    }
}
