use crate::{
    common::{format_tournament_datetime, render_text_prop},
    i18n::*,
};
use chrono::{DateTime, Duration, Local, LocalResult, NaiveDateTime, Utc};
use leptos::prelude::*;

fn resolve_local_draft(date: NaiveDateTime) -> LocalResult<DateTime<Local>> {
    let local = match date.and_local_timezone(Local) {
        LocalResult::Single(local) => local,
        other => return other,
    };
    let utc = local.to_utc();
    // On wasm, JavaScript's Date chooses an occurrence during a fold and
    // normalizes gaps. Verify the wall time and alternative nearby offsets.
    if utc.with_timezone(&Local).naive_local() != date {
        return LocalResult::None;
    }
    for day in [-1, 1] {
        let Some(neighbor) = utc.checked_add_signed(Duration::days(day)) else {
            continue;
        };
        let offset = neighbor.with_timezone(&Local).offset().local_minus_utc();
        let Some(alternative) = date
            .and_utc()
            .checked_sub_signed(Duration::seconds(i64::from(offset)))
        else {
            continue;
        };
        if alternative != utc && alternative.with_timezone(&Local).naive_local() == date {
            return LocalResult::Ambiguous(local, alternative.with_timezone(&Local));
        }
    }
    LocalResult::Single(local)
}

#[derive(Clone, Copy)]
enum DraftError {
    Incomplete,
    BeforeMinimum(DateTime<Local>),
    AfterMaximum(DateTime<Local>),
    Ambiguous,
    Missing,
}

impl DraftError {
    fn label(self) -> String {
        let i18n = use_i18n();
        match self {
            Self::Incomplete => {
                t_string!(i18n, tournaments.ui.enter_a_complete_date_and_time).to_string()
            }
            Self::BeforeMinimum(min) => t_string!(
                i18n,
                tournaments.ui.choose_a_time_on_or_after,
                value1 = format_tournament_datetime(min.to_utc())
            )
            .to_string(),
            Self::AfterMaximum(max) => t_string!(
                i18n,
                tournaments.ui.choose_a_time_on_or_before,
                value1 = format_tournament_datetime(max.to_utc())
            )
            .to_string(),
            Self::Ambiguous => t_string!(
                i18n,
                tournaments
                    .ui
                    .this_local_time_occurs_twice_when_the_clocks_change_choose_another_time
            )
            .to_string(),
            Self::Missing => t_string!(i18n, tournaments.ui.nonexistent_local_time).to_string(),
        }
    }
}

fn parse_draft(
    draft: &str,
    min: DateTime<Local>,
    max: DateTime<Local>,
) -> Result<DateTime<Utc>, DraftError> {
    if draft.is_empty() {
        return Err(DraftError::Incomplete);
    }
    let date = NaiveDateTime::parse_from_str(draft, "%Y-%m-%dT%H:%M")
        .map_err(|_| DraftError::Incomplete)?;
    match resolve_local_draft(date) {
        LocalResult::Single(local) if local < min => Err(DraftError::BeforeMinimum(min)),
        LocalResult::Single(local) if local > max => Err(DraftError::AfterMaximum(max)),
        LocalResult::Single(local) => Ok(local.to_utc()),
        LocalResult::Ambiguous(_, _) => Err(DraftError::Ambiguous),
        LocalResult::None => Err(DraftError::Missing),
    }
}

#[component]
pub fn DateTimePicker(
    input_id: String,
    #[prop(into)] text: TextProp,
    min: DateTime<Local>,
    max: DateTime<Local>,
    success_callback: Callback<(DateTime<Utc>,), ()>,
    #[prop(optional)] value: Option<DateTime<Local>>,
    #[prop(optional)] draft_valid: Option<RwSignal<bool>>,
) -> impl IntoView {
    let value = value.unwrap_or_else(|| (min + Duration::minutes(1)).min(max));
    let initial_draft = value.format("%Y-%m-%dT%H:%M").to_string();
    let initial_result = parse_draft(&initial_draft, min, max);
    let draft_valid = draft_valid.unwrap_or_else(|| RwSignal::new(initial_result.is_ok()));
    draft_valid.set(initial_result.is_ok());
    let error = RwSignal::new(initial_result.err());
    let show_error = RwSignal::new(false);
    let input_id = StoredValue::new(input_id);
    let validate =
        Callback::new(
            move |displayed: String| match parse_draft(&displayed, min, max) {
                Ok(time) => {
                    draft_valid.set(true);
                    error.set(None);
                    success_callback.run((time,));
                }
                Err(message) => {
                    draft_valid.set(false);
                    error.set(Some(message));
                }
            },
        );
    view! {
        <label class="flex flex-col gap-1.5 min-w-0" for=move || input_id.get_value()>
            <span class="ui-field-label">{render_text_prop(text)}</span>
            <input
                type="datetime-local"
                id=move || input_id.get_value()
                name="start-time"
                class="w-full min-w-0 ui-field-input"
                prop:min=move || min.format("%Y-%m-%dT%H:%M").to_string()
                prop:max=move || max.format("%Y-%m-%dT%H:%M").to_string()
                value=initial_draft
                on:input=move |evt| validate.run(event_target_value(&evt))
                on:change=move |_| show_error.set(true)
                on:blur=move |_| show_error.set(true)
            />
            <Show when=move || {
                show_error.get()
            }>
                {move || {
                    error
                        .get()
                        .map(|message| {
                            view! { <span class="ui-field-error">{move || message.label()}</span> }
                        })
                }}
            </Show>
        </label>
    }
}
