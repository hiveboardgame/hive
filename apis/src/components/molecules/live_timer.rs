use crate::{
    common::format_clock_time_remaining,
    i18n::*,
    providers::{
        game_state::GameStateStore,
        timer::TimerSignal,
        ApiRequestsProvider,
        AuthContext,
        SoundType,
        Sounds,
    },
};
use chrono::{DateTime, TimeDelta, Utc};
use hive_lib::Color;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use leptos_use::{
    use_interval_fn_with_options,
    utils::Pausable,
    watch_with_options,
    whenever_with_options,
    UseIntervalFnOptions,
    WatchOptions,
};
use shared_types::GameId;
use std::time::Duration;

const REPORT_AGAIN_AFTER: TimeDelta = TimeDelta::seconds(1);

#[component]
pub fn LiveTimer(side: Signal<Color>, #[prop(optional)] compact: bool) -> impl IntoView {
    let i18n = use_i18n();
    let game_state = expect_context::<GameStateStore>();
    let sounds = expect_context::<Sounds>();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let user_color = game_state.user_color_as_signal(auth_context.identity);
    let timer = expect_context::<TimerSignal>().signal;
    let tick_rate = Duration::from_millis(100);
    let Pausable { pause, resume, .. } = use_interval_fn_with_options(
        move || {
            timer.update(|t| {
                if t.turn.is_multiple_of(2) {
                    t.white_time_left = t
                        .white_time_left
                        .map(|t| t.checked_sub(tick_rate).unwrap_or_default());
                } else {
                    t.black_time_left = t
                        .black_time_left
                        .map(|t| t.checked_sub(tick_rate).unwrap_or_default());
                };
            })
        },
        100,
        UseIntervalFnOptions::default().immediate(false),
    );
    let should_resume = Signal::derive(move || {
        timer.with(|t| {
            t.ordinary_clock_running && (side() == Color::White) == (t.turn.is_multiple_of(2))
        })
    });
    let time_is_zero = Signal::derive(move || timer.with(|t| t.time_left(side()).is_zero()));
    let user_needs_warning = Signal::derive(move || {
        user_color().is_some_and(|color| {
            timer.with(|t| {
                t.warning_trigger().is_some_and(|trigger_at| {
                    if color == side() && t.ordinary_clock_running {
                        t.time_left(color) < trigger_at
                    } else {
                        false
                    }
                })
            })
        })
    });
    let should_refresh_warning = Signal::derive(move || {
        user_color().is_some_and(|color| {
            timer.with(|t| {
                t.warning_refresh().is_some_and(|refresh_at| {
                    if color == side() && t.ordinary_clock_running {
                        t.time_left(color) > refresh_at
                    } else {
                        false
                    }
                })
            })
        })
    });

    //For styling timer updated by history navigation
    let timed_out = Signal::derive(move || {
        timer.with(|t| {
            if side() == Color::White {
                t.white_timed_out
            } else {
                t.black_timed_out
            }
        })
    });

    let _ = watch_with_options(
        should_resume,
        move |v, _, _| {
            if *v {
                resume();
            } else {
                pause();
            }
        },
        WatchOptions::default().immediate(true),
    );

    // Reports repeat while the clock reads zero, and each locks the game row a late move may be
    // waiting for; the server rechecks a flag it cannot settle yet.
    let last_report = StoredValue::new(None::<(GameId, usize, DateTime<Utc>)>);
    let _ = whenever_with_options(
        move || time_is_zero() && timer().ordinary_clock_running,
        move |_, _, _| {
            let (game_id, turn, now) = (game_id(), timer.with_untracked(|t| t.turn), Utc::now());
            let reported = last_report.with_value(|last| {
                last.as_ref().is_some_and(|(game, at_turn, at)| {
                    *game == game_id && *at_turn == turn && now - *at < REPORT_AGAIN_AFTER
                })
            });
            if !reported {
                last_report.set_value(Some((game_id.clone(), turn, now)));
                api.get().game_check_time(&game_id, turn);
            }
        },
        WatchOptions::default().immediate(true),
    );

    let _ = whenever_with_options(
        user_needs_warning,
        move |_, _, issued| {
            let issued = issued.unwrap_or_default();
            if issued {
                !should_refresh_warning()
            } else {
                sounds.play_sound(SoundType::LowTime);
                true
            }
        },
        WatchOptions::default().immediate(true),
    );

    let timer_text_class = if compact {
        "px-1 font-mono text-[0.95rem] leading-none tabular-nums whitespace-nowrap"
    } else {
        "text-xl md:text-2xl lg:text-4xl"
    };

    view! {
        <div class=move || {
            format!(
                "flex resize h-full min-w-0 select-none items-center justify-center {timer_text_class} {}",
                if timed_out() { "bg-ladybug-red" } else { "" },
            )
        }>
            {move || {
                timer
                    .with(|t| {
                        let time_left = t.time_left(side());
                        format_clock_time_remaining(i18n, t.time_mode, time_left)
                    })
            }}

        </div>
    }
}
