use crate::common::ScheduleAction;
use crate::components::atoms::date_time_picker::DateTimePicker;
use crate::providers::ApiRequests;
use crate::responses::ScheduleResponse;
use chrono::{DateTime, Duration, Local, Utc};
use leptos::*;
use shared_types::GameId;
use uuid::Uuid;

#[component]
pub fn GameDateControls(player_id: Uuid, schedule: ScheduleResponse) -> impl IntoView {
    let start_date = schedule.start_t;
    let agreed = schedule.agreed;
    let id = schedule.id;
    let proposer_id = schedule.proposer_id;
    let formated_game_date = move |time: DateTime<Utc>| {
        let to_date = time - Utc::now();

        let agreed_str = if agreed { "To play" } else { "Proposed" };
        format!(
            "{} in {} days, {} hours, {} minutes ({})",
            agreed_str,
            to_date.num_days(),
            to_date.num_hours() % 24,
            to_date.num_minutes() % 60,
            time.with_timezone(&Local).format("%m-%d %H:%M")
        )
    };
    let accept = Callback::from(move |id| {
        let api = ApiRequests::new();
        api.schedule_action(ScheduleAction::Accept(id));
    });
    view! {
        <div class="flex justify-center px-2">
            <div class="flex items-center">{formated_game_date(start_date)}</div>
            <Show when=move || !agreed && proposer_id != player_id>
                <button
                    on:click=move |_| accept(id)
                    class="px-2 py-2 m-1 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                >

                    "Accept"
                </button>
            </Show>
            <button
                on:click=move |_| {
                    let api = ApiRequests::new();
                    api.schedule_action(ScheduleAction::Cancel(id));
                }

                class="px-2 py-2 m-1 text-white rounded transition-transform duration-300 transform bg-ladybug-red hover:bg-red-400 active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
            >
                {(if proposer_id == player_id { "Cancel" } else { "Reject" }).to_string()}
            </button>
        </div>
    }
}

#[component]
pub fn ProposeDateControls(game_id: GameId) -> impl IntoView {
    let selected_time = RwSignal::new(Utc::now() + Duration::minutes(10));
    let propose = Callback::from(move |date| {
        let api = ApiRequests::new();
        api.schedule_action(ScheduleAction::Propose(date, game_id.clone()));
    });
    view! {
        <div class="flex justify-center px-2">
            <DateTimePicker
                text=""
                min=Local::now() + Duration::minutes(10)
                max=Local::now() + Duration::weeks(12)
                success_callback=Callback::from(move |utc| {
                    selected_time.set(utc);
                })

                failure_callback=Callback::from(|_| { logging::log!("Failure to set date") })
            />

            <button
                class="px-2 py-2 m-1 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed disabled:hover:bg-transparent"
                on:click=move |_| propose(selected_time.get())
            >

                "Propose Date"
            </button>
        </div>
    }
}
