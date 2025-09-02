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
    let div_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let local_time = start_time.with_timezone(&Local);
    let notification_text = format!(
        "{proposer_username} proposed a game time: {}",
        local_time.format("%Y-%m-%d %H:%M UTC%Z")
    );

    let accept = move |_| {
        let api = api.get();
        api.schedule_action(ScheduleAction::Accept(schedule_id.get_value()));
        notifications.schedule_proposals.update(|proposals| {
            proposals.remove(&schedule_id.get_value());
        });
    };

    let decline = move |_| {
        let api = api.get();
        api.schedule_action(ScheduleAction::Cancel(schedule_id.get_value()));
        notifications.schedule_proposals.update(|proposals| {
            proposals.remove(&schedule_id.get_value());
        });
    };

    let dismiss = move |_| {
        notifications.schedule_proposals.update(|proposals| {
            proposals.remove(&schedule_id.get_value());
        });
    };

    view! {
        <div class="flex flex-col p-2 w-full dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <div class="flex justify-between items-center mb-2 w-full">
                <div class=div_class>
                    <div>{notification_text}</div>
                    <div class="text-sm text-gray-600 dark:text-gray-400">
                        <a
                            href=format!("/tournament/{}", &tournament_id.to_string())
                            class="text-blue-600 dark:text-blue-400 hover:underline"
                        >
                            "View Tournament"
                        </a>
                    </div>
                </div>
                <button
                    title="Dismiss"
                    on:click=dismiss
                    class="z-20 p-1 mx-2 text-white bg-gray-500 rounded transition-transform duration-300 hover:bg-gray-400 active:scale-95"
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                </button>
            </div>
            <div class="flex gap-2 justify-center">
                <button
                    on:click=accept
                    class="px-3 py-1 text-white bg-green-600 rounded transition-transform duration-300 hover:bg-green-500 active:scale-95"
                >
                    "Accept"
                </button>
                <button
                    on:click=decline
                    class="px-3 py-1 text-white bg-red-600 rounded transition-transform duration-300 hover:bg-red-500 active:scale-95"
                >
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
    let div_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let local_time = start_time.with_timezone(&Local);
    let notification_text = format!(
        "{accepter_username} accepted your proposed game time: {}",
        local_time.format("%Y-%m-%d %H:%M UTC%Z")
    );

    let mark_seen_action = ServerAction::<MarkScheduleSeen>::new();

    let dismiss = move |_| {
        notifications.schedule_acceptances.update(|acceptances| {
            acceptances.remove(&schedule_id.get_value());
        });
    };

    view! {
        <div class="flex flex-col p-2 w-full dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <div class="flex justify-between items-center mb-2 w-full">
                <div class=div_class>
                    <div>{notification_text}</div>
                    <div class="text-sm text-gray-600 dark:text-gray-400">
                        <a
                            href=format!("/tournament/{}", &tournament_id.to_string())
                            class="text-blue-600 dark:text-blue-400 hover:underline"
                        >
                            "View Tournament:"
                            {tournament_name}
                        </a>
                    </div>
                </div>
                <ActionForm action=mark_seen_action on:submit=dismiss>
                    <input
                        type="hidden"
                        name="schedule_id"
                        value=schedule_id.get_value().to_string()
                    />
                    <button
                        type="submit"
                        title="Dismiss"
                        class="z-50 p-1 mx-2 text-white bg-gray-500 rounded transition-transform duration-300 hover:bg-gray-400 active:scale-95"
                    >
                        <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                    </button>
                </ActionForm>
            </div>
        </div>
    }
}
