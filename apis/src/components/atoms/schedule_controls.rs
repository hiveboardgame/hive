use crate::{
    common::{with_class, ScheduleAction},
    components::atoms::date_time_picker::DateTimePicker,
    i18n::{t, t_string, use_i18n},
    providers::ApiRequestsProvider,
    responses::ScheduleResponse,
};
use chrono::{DateTime, Duration, Local, Utc};
use leptos::{callback::Callback, prelude::*};
use shared_types::GameId;
use uuid::Uuid;

#[component]
pub fn GameDateControls(player_id: Uuid, schedule: ScheduleResponse) -> impl IntoView {
    let i18n = use_i18n();
    let start_date = schedule.start_t;
    let agreed = schedule.agreed;
    let id = schedule.id;
    let proposer_id = schedule.proposer_id;
    let api = expect_context::<ApiRequestsProvider>().0;
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
        let api = api.get();
        api.schedule_action(ScheduleAction::Accept(id));
    });
    view! {
        <div class=with_class(
            if agreed { "ui-notice" } else { "ui-warning-notice" },
            "flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between",
        )>
            <div class=format!(
                "flex min-w-0 items-center {}",
                if agreed { "font-bold" } else { "" },
            )>{formatted_game_date(start_date)}</div>
            <Show when=move || !agreed && proposer_id != player_id>
                <button
                    on:click=move |_| accept.run((id,))
                    class="m-1 ui-button ui-button-success ui-button-sm"
                >

                    {t!(i18n, common.accept)}
                </button>
            </Show>
            <button
                on:click=move |_| {
                    let api = api.get();
                    api.schedule_action(ScheduleAction::Cancel(id));
                }

                class="m-1 ui-button ui-button-danger ui-button-sm"
            >
                {move || {
                    if proposer_id == player_id || agreed {
                        t_string!(i18n, common.cancel)
                    } else {
                        t_string!(i18n, common.reject)
                    }
                }}
            </button>
        </div>
    }
}

#[component]
pub fn ProposeDateControls(game_id: GameId) -> impl IntoView {
    let selected_time = RwSignal::new(Utc::now() + Duration::minutes(10));
    let api = expect_context::<ApiRequestsProvider>().0;
    let propose = Callback::from(move |date| {
        let api = api.get();
        api.schedule_action(ScheduleAction::Propose(date, game_id.clone()));
    });
    let callback = Callback::from(move |utc: DateTime<Utc>| {
        selected_time.set(utc);
    });
    view! {
        <div class="flex flex-col gap-2 p-2 sm:flex-row sm:justify-between sm:items-center">
            <DateTimePicker
                text=""
                min=Local::now() + Duration::minutes(10)
                max=Local::now() + Duration::weeks(12)
                success_callback=callback
            />

            <button
                class="m-1 ui-button ui-button-primary ui-button-sm"
                on:click=move |_| propose.run((selected_time.get(),))
            >

                "Propose Date"
            </button>
        </div>
    }
}
