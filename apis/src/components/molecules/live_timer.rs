use crate::{
    common::FlashStyle,
    hooks::flash_pulse::use_flash_pulse,
    providers::{
        game_state::{GameStateStore, GameStateStoreFields},
        timer::TimerSignal,
        ApiRequestsProvider,
        AuthContext,
    },
};
use hive_lib::{Color, GameStatus};
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

#[component]
pub fn LiveTimer(side: Signal<Color>, #[prop(optional)] compact: bool) -> impl IntoView {
    let game_state = expect_context::<GameStateStore>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let params = use_params_map();
    let game_id = move || {
        params
            .get()
            .get("nanoid")
            .map(|s| GameId(s.to_owned()))
            .unwrap_or_default()
    };
    let game_response = game_state.game_response();
    let in_progress = Memo::new(move |_| {
        game_response.with(|game_response| {
            game_response
                .as_ref()
                .is_some_and(|gr| gr.game_status == GameStatus::InProgress)
        })
    });
    let auth_context = expect_context::<AuthContext>();
    let user_color = game_state.user_color_as_signal(auth_context.identity);
    let timer_flashing = use_flash_pulse(FlashStyle::Timer);
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
            in_progress() && (side() == Color::White) == (t.turn.is_multiple_of(2)) && !t.finished
        })
    });
    let time_is_zero = Signal::derive(move || timer.with(|t| t.time_left(side()).is_zero()));
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

    let _ = whenever_with_options(
        move || time_is_zero() && !timer().finished,
        move |_, _, _| {
            // When time runs out declare winner and style timer that ran out
            let api = api.get();
            api.game_check_time(&game_id());
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
                "flex resize h-full min-w-0 select-none items-center justify-center {timer_text_class} {} {}",
                if timed_out() { "bg-ladybug-red" } else { "" },
                if timer_flashing() && user_color() == Some(side()) {
                    "warning-flash-timer"
                } else {
                    ""
                },
            )
        }>
            {move || {
                timer
                    .with(|t| {
                        let time_left = t.time_left(side());
                        t.time_mode.time_remaining(time_left)
                    })
            }}

        </div>
    }
}
