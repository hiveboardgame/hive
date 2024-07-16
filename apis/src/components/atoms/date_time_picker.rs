use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use leptos::*;

#[component]
pub fn DateTimePicker(
    text: &'static str,
    min: DateTime<Local>,
    max: DateTime<Local>,
    success_callback: Callback<DateTime<Utc>>,
    failure_callback: Callback<()>,
) -> impl IntoView {
    view! {
        <label>{text}</label>
        <input
            type="datetime-local"
            id="start-time"
            name="start-time"
            attr:min=move || { min.format("%Y-%m-%dT%H:%M").to_string() }

            attr:max=move || { max.format("%Y-%m-%dT%H:%M").to_string() }

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
                        success_callback(utc);
                    }
                } else {
                    failure_callback(());
                }
            }
        />
    }
}
