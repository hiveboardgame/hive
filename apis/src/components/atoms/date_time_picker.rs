use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use leptos::prelude::*;

#[component]
pub fn DateTimePicker(
    text: &'static str,
    min: DateTime<Local>,
    max: DateTime<Local>,
    success_callback: Callback<(DateTime<Utc>,), ()>,
    #[prop(optional)] failure_callback: Option<Callback<(), ()>>,
) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1.5">
            <span class="ui-field-label">{text}</span>
            <input
                type="datetime-local"
                id="start-time"
                name="start-time"
                class="ui-field-input"
                prop:min=move || { min.format("%Y-%m-%dT%H:%M").to_string() }

                prop:max=move || { max.format("%Y-%m-%dT%H:%M").to_string() }

                value=(min + Duration::minutes(1)).format("%Y-%m-%dT%H:%M").to_string()
                on:input=move |evt| {
                    if let Ok(date) = NaiveDateTime::parse_from_str(
                        &event_target_value(&evt),
                        "%Y-%m-%dT%H:%M",
                    ) {
                        let dt = Local::now();
                        let offset = dt.offset();
                        if let chrono::LocalResult::Single(local) = NaiveDateTime::and_local_timezone(
                            &date,
                            *offset,
                        ) {
                            let utc = local.to_utc();
                            success_callback.run((utc,));
                        }
                    } else if let Some(failure_callback) = failure_callback {
                        failure_callback.run(());
                    }
                }
            />
        </label>
    }
}
