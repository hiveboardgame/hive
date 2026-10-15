use crate::i18n::*;
use chrono::{DateTime, Local, Utc};

pub(crate) fn format_local_datetime(_locale: Locale, date_time: DateTime<Utc>) -> String {
    format_tournament_datetime(date_time)
}

pub(crate) fn format_tournament_datetime(date_time: DateTime<Utc>) -> String {
    date_time
        .with_timezone(&Local)
        // TODO: i18n once copy is approved.
        .format("%-d %b %Y, %H:%M UTC%:z")
        .to_string()
}
