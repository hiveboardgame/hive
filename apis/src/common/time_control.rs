use crate::i18n::*;
use leptos_i18n::I18nContext;
use shared_types::{Clock, CorrespondenceClock, RealtimeClock};

pub(crate) fn format_compact_duration(seconds: u32) -> String {
    if seconds == 0 {
        String::from("0s")
    } else if seconds.is_multiple_of(3600) {
        format!("{}h", seconds / 3600)
    } else if seconds.is_multiple_of(60) {
        format!("{}m", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}

pub(crate) fn format_time_control(
    i18n: I18nContext<Locale, I18nKeys>,
    time_control: Option<Clock>,
) -> String {
    match time_control {
        None => t_string!(i18n, game.speeds.untimed).to_string(),
        Some(Clock::Realtime(clock)) => format!(
            "{} + {}",
            format_compact_duration(clock.base_seconds.get()),
            format_compact_duration(clock.increment_seconds),
        ),
        Some(Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each })) => {
            if seconds_each.get().is_multiple_of(86400) {
                let days = i32::try_from(seconds_each.get() / 86400)
                    .expect("u32 seconds contain an i32 number of days");
                t_string!(i18n, game.time_mode.correspondence.days_side, count = days).to_string()
            } else {
                t_string!(
                    i18n,
                    game.time_mode.correspondence.duration_side,
                    duration = format_compact_duration(seconds_each.get())
                )
                .to_string()
            }
        }
        Some(Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move })) => {
            if seconds_per_move.get().is_multiple_of(86400) {
                let days = i32::try_from(seconds_per_move.get() / 86400)
                    .expect("u32 seconds contain an i32 number of days");
                t_string!(i18n, game.time_mode.correspondence.days_move, count = days).to_string()
            } else {
                t_string!(
                    i18n,
                    game.time_mode.correspondence.duration_move,
                    duration = format_compact_duration(seconds_per_move.get())
                )
                .to_string()
            }
        }
    }
}

fn compact_realtime_clock(clock: RealtimeClock) -> String {
    let seconds = clock.base_seconds.get();
    let base = if seconds.is_multiple_of(60) {
        (seconds / 60).to_string()
    } else {
        format_compact_duration(seconds)
    };
    format!("{base}+{}", clock.increment_seconds)
}

#[derive(Clone, Copy)]
pub(crate) enum TournamentClockStyle {
    Compact,
    Detailed,
    PhaseSummary,
}

pub(crate) fn format_tournament_clock(
    i18n: I18nContext<Locale, I18nKeys>,
    clock: Clock,
    style: TournamentClockStyle,
) -> String {
    let (seconds, per_move) = match clock {
        Clock::Realtime(clock) => return compact_realtime_clock(clock),
        Clock::Correspondence(CorrespondenceClock::TotalTimeEach { seconds_each }) => {
            (seconds_each.get(), false)
        }
        Clock::Correspondence(CorrespondenceClock::DaysPerMove { seconds_per_move }) => {
            (seconds_per_move.get(), true)
        }
    };
    let days = seconds / 86400;
    let duration = match (style, seconds.is_multiple_of(86400)) {
        (TournamentClockStyle::Compact, true) if days == 1 => {
            t_string!(i18n, tournaments.row.one_day).to_string()
        }
        (TournamentClockStyle::Compact, true) => t_string!(
            i18n,
            home.custom_game.mode.correspondence.time,
            count = days
        )
        .to_string(),
        (TournamentClockStyle::Detailed | TournamentClockStyle::PhaseSummary, true) => {
            t_string!(i18n, tournaments.ui.day_count, count = days).to_string()
        }
        (_, false) => format_compact_duration(seconds),
    };
    match (style, per_move) {
        (TournamentClockStyle::Compact, true) => {
            t_string!(i18n, tournaments.row.duration_per_move, duration = duration).to_string()
        }
        (TournamentClockStyle::Compact, false) => {
            t_string!(i18n, tournaments.row.duration_per_side, duration = duration).to_string()
        }
        (TournamentClockStyle::Detailed | TournamentClockStyle::PhaseSummary, true) => t_string!(
            i18n,
            game.time_mode.correspondence.duration_move,
            duration = duration
        )
        .to_string(),
        (TournamentClockStyle::Detailed, false) => {
            t_string!(i18n, tournaments.ui.time_per_player, value1 = duration).to_string()
        }
        (TournamentClockStyle::PhaseSummary, false) => t_string!(
            i18n,
            tournaments.creation.elimination.summary.time_each,
            time = duration
        )
        .to_string(),
    }
}
