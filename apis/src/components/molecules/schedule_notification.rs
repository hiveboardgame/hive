use crate::{
    common::ScheduleAction,
    providers::{ApiRequestsProvider, NotificationContext},
};
use chrono::{DateTime, Utc};
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
    let notification_text = format!(
        "{proposer_username} proposed a game time: {}",
        start_time.format("%Y-%m-%d %H:%M UTC")
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
        <div class="flex flex-col p-2 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light w-full">
            <div class="flex items-center justify-between mb-2 w-full">
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
                    class="z-20 p-1 mx-2 text-white bg-gray-500 rounded transition-transform duration-300 transform hover:bg-gray-400 active:scale-95"
                >
                    <Icon icon=icondata::IoCloseSharp attr:class="w-4 h-4" />
                </button>
            </div>
            <div class="flex gap-2 justify-center">
                <button
                    on:click=accept
                    class="px-3 py-1 text-white bg-green-600 rounded transition-transform duration-300 transform hover:bg-green-500 active:scale-95"
                >
                    "Accept"
                </button>
                <button
                    on:click=decline
                    class="px-3 py-1 text-white bg-red-600 rounded transition-transform duration-300 transform hover:bg-red-500 active:scale-95"
                >
                    "Decline"
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn AcceptanceNotification(
    schedule_id: Uuid,
    accepter_username: String,
    tournament_id: TournamentId,
    start_time: DateTime<Utc>,
) -> impl IntoView {
    let notifications = expect_context::<NotificationContext>();
    let schedule_id = StoredValue::new(schedule_id);
    let div_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let notification_text = format!(
        "{accepter_username} accepted your proposed game time: {}",
        start_time.format("%Y-%m-%d %H:%M UTC")
    );

    let dismiss = move |_| {
        notifications.schedule_acceptances.update(|acceptances| {
            acceptances.remove(&schedule_id.get_value());
        });
    };

    view! {
        <div class="flex flex-col p-2 dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light w-full">
            <div class="flex items-center justify-between mb-2 w-full">
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
                    class="z-20 p-1 mx-2 text-white bg-gray-500 rounded transition-transform duration-300 transform hover:bg-gray-400 active:scale-95"
                >
                    <Icon icon=icondata::IoCloseSharp attr:class="w-4 h-4" />
                </button>
            </div>
        </div>
    }
}
