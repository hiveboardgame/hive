use crate::i18n::*;
use chrono::{DateTime, Local, Utc};

pub(crate) fn format_local_datetime(locale: Locale, date_time: DateTime<Utc>) -> String {
    let local = date_time.with_timezone(&Local);
    td_string!(
        locale,
        common.datetime,
        year = local.format("%Y").to_string(),
        month = local.format("%m").to_string(),
        day = local.format("%d").to_string(),
        time = local.format("%H:%M").to_string(),
        offset = local.format("%:z").to_string(),
    )
    .to_string()
}

pub(crate) fn format_tournament_datetime(date_time: DateTime<Utc>) -> String {
    format_local_datetime(use_i18n().get_locale(), date_time)
}

#[cfg(feature = "ssr")]
pub(crate) fn format_utc_datetime(locale: Locale, date_time: DateTime<Utc>) -> String {
    td_string!(
        locale,
        common.datetime,
        year = date_time.format("%Y").to_string(),
        month = date_time.format("%m").to_string(),
        day = date_time.format("%d").to_string(),
        time = date_time.format("%H:%M").to_string(),
        offset = "+00:00",
    )
    .to_string()
}
