use crate::common::ScheduleAction;
use crate::components::atoms::date_time_picker::DateTimePicker;
use crate::responses::ScheduleResponse;
use crate::websocket::new_style::client::ClientApi;
use chrono::{DateTime, Duration, Local, Utc};
use leptos::callback::Callback;
use leptos::prelude::*;
use leptos::task::spawn_local;
use shared_types::GameId;
use uuid::Uuid;

#[component]
pub fn GameDateControls(player_id: Uuid, schedule: ScheduleResponse) -> impl IntoView {
    let start_date = schedule.start_t;
    let agreed = schedule.agreed;
    let id = schedule.id;
    let proposer_id = schedule.proposer_id;
    let api = expect_context::<ClientApi>();
    let formatted_game_date = move |time: DateTime<Utc>| {
        let to_date = time - Utc::now();

        let agreed_str = if agreed { "To play" } else { "Proposed" };
        format!(
            "{} in {} days, {} hours, {} minutes ({})",
            agreed_str,
            to_date.num_days(),
            to_date.num_hours() % 24,
            to_date.num_minutes() % 60,
            time.with_timezone(&Local).format("%m-%d %H:%M %Z")
        )
    };
    let accept = Callback::from(move |id| {
        spawn_local(async move {
            api.schedule_action(ScheduleAction::Accept(id)).await;
        });
    });
    view! {
        <div class={format!(
            "flex justify-between p-2 rounded text-gray-900 dark:text-gray-100 {}",
            if agreed {
                "bg-green-200 dark:bg-green-900"
            } else {
                "bg-yellow-200 dark:bg-yellow-600"
            }
        )}>
            <div class=format!(
                "flex items-center {}",
                if agreed { "font-bold" } else { "" },
            )>{formatted_game_date(start_date)}</div>
            <Show when=move || !agreed && proposer_id != player_id>
                <button
                    on:click=move |_| accept.run((id,))
                    class="px-2 py-2 m-1 text-white rounded transition-transform duration-300 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
                >

                    "Accept"
                </button>
            </Show>
            <button
                on:click=move |_| {
                    spawn_local(async move {
                        api.schedule_action(ScheduleAction::Cancel(id)).await;
                    })
                }

                class="px-2 py-2 m-1 text-white rounded transition-transform duration-300 bg-ladybug-red hover:bg-red-400 active:scale-95"
            >
                {(if proposer_id == player_id || agreed { "Cancel" } else { "Reject" }).to_string()}
            </button>
        </div>
    }
}

#[component]
pub fn ProposeDateControls(game_id: GameId) -> impl IntoView {
    let selected_time = RwSignal::new(Utc::now() + Duration::minutes(10));
    let api = expect_context::<ClientApi>();
    let propose = Callback::from(move |date| {
        let gid = game_id.clone();
        spawn_local(async move {
            api.schedule_action(ScheduleAction::Propose(date, gid.clone()))
                .await;
        });
    });
    let callback = Callback::from(move |utc: DateTime<Utc>| {
        selected_time.set(utc);
    });
    view! {
        <div class="flex justify-between p-2">
            <DateTimePicker
                text=""
                min=Local::now() + Duration::minutes(10)
                max=Local::now() + Duration::weeks(12)
                success_callback=callback
            />

            <button
                class="px-2 py-2 m-1 text-white rounded transition-transform duration-300 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
                on:click=move |_| propose.run((selected_time.get(),))
            >

                "Propose Date"
            </button>
        </div>
    }
}
