use crate::{
    common::ScheduleAction,
    functions::schedules::MarkScheduleSeen,
    providers::{ApiRequestsProvider, NotificationContext},
};
use chrono::{DateTime, Local, Utc};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;
use uuid::Uuid;

#[component]
pub fn ProposalNotification(
    schedule_id: Uuid,
    proposer_username: String,
    tournament_id: TournamentId,
    start_time: DateTime<Utc>,
) -> impl IntoView {
    let notifications = expect_context::<NotificationContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let schedule_id = StoredValue::new(schedule_id);
    let local_time = start_time.with_timezone(&Local);
    let notification_text = format!(
        "{proposer_username} proposed a game time: {}",
        local_time.format("%Y-%m-%d %H:%M UTC%Z")
    );

    let accept = move |_| {
        let api = api.get();
        api.schedule_action(ScheduleAction::Accept(schedule_id.get_value()));
        notifications.schedule_notification_remove(schedule_id.get_value());
    };

    let decline = move |_| {
        let api = api.get();
        api.schedule_action(ScheduleAction::Cancel(schedule_id.get_value()));
        notifications.schedule_notification_remove(schedule_id.get_value());
    };

    let dismiss = move |_| {
        notifications.schedule_notification_remove(schedule_id.get_value());
    };

    view! {
        <div class="flex-col gap-2 items-stretch ui-notification-item">
            <div class="flex gap-2 items-start w-full">
                <div class="ui-notification-item-body">
                    <div class="whitespace-normal ui-notification-title">{notification_text}</div>
                    <div class="text-sm text-gray-600 dark:text-gray-400">
                        <a
                            href=format!("/tournament/{}", &tournament_id.to_string())
                            class="ui-text-link"
                        >
                            "View Tournament"
                        </a>
                    </div>
                </div>
                <button
                    title="Dismiss"
                    on:click=dismiss
                    class="z-20 ui-button ui-button-ghost ui-button-icon"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                </button>
            </div>
            <div class="flex gap-2 justify-end">
                <button on:click=accept class="ui-button ui-button-success ui-button-sm">
                    "Accept"
                </button>
                <button on:click=decline class="ui-button ui-button-danger ui-button-sm">
                    "Decline"
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn AcceptanceNotification(
    tournament_name: String,
    schedule_id: Uuid,
    accepter_username: String,
    tournament_id: TournamentId,
    start_time: DateTime<Utc>,
) -> impl IntoView {
    let notifications = expect_context::<NotificationContext>();
    let schedule_id = StoredValue::new(schedule_id);
    let local_time = start_time.with_timezone(&Local);
    let notification_text = format!(
        "{accepter_username} accepted your proposed game time: {}",
        local_time.format("%Y-%m-%d %H:%M UTC%Z")
    );

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
                        href=format!("/tournament/{}", &tournament_id.to_string())
                        class="ui-text-link"
                    >
                        "View Tournament:"
                        {tournament_name}
                    </a>
                </div>
            </div>
            <ActionForm action=mark_seen_action on:submit=dismiss>
                <input type="hidden" name="schedule_id" value=schedule_id.get_value().to_string() />
                <button
                    type="submit"
                    title="Dismiss"
                    class="z-50 ui-button ui-button-ghost ui-button-icon"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                </button>
            </ActionForm>
        </div>
    }
}
