use crate::providers::{
    game_state::GameStateSignal, timer::TimerSignal, ApiRequestsProvider, AuthContext, SoundType,
    Sounds,
};
use hive_lib::{Color, GameStatus};
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use leptos_use::{
    use_interval_fn_with_options, utils::Pausable, watch_with_options, whenever_with_options,
    UseIntervalFnOptions, WatchOptions,
};
use shared_types::GameId;
use std::time::Duration;

#[component]
pub fn LiveTimer(side: Signal<Color>) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
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
    let user_id = Signal::derive(move || {
        auth_context
            .user
            .with_untracked(|a| a.as_ref().map(|user| user.id))
    });
    let user_color = game_state.user_color_as_signal(user_id);
    let in_progress = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .is_some_and(|gr| gr.game_status == GameStatus::InProgress)
    });
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
    let user_needs_warning = Signal::derive(move || {
        user_color().is_some_and(|color| {
            timer.with(|t| {
                t.warning_trigger().is_some_and(|trigger_at| {
                    if color == side() && !t.finished {
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
                    if color == side() && !t.finished {
                        t.time_left(color) > refresh_at
                    } else {
                        false
                    }
                })
            })
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

    view! {
        <div
            id="timer"
            class=move || {
                format!(
                    "flex resize h-full select-none items-center justify-center text-xl md:text-2xl lg:text-4xl {}",
                    if time_is_zero() { "bg-ladybug-red" } else { "" },
                )
            }
        >
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
