use crate::providers::{
    game_state::GameStateSignal, timer::TimerSignal, ApiRequests, AuthContext, SoundType,
    SoundsSignal,
};
use hive_lib::{Color, GameStatus};
use lazy_static::lazy_static;
use leptos::*;
use leptos_router::RouterContext;
use leptos_use::{use_interval_fn_with_options, utils::Pausable, UseIntervalFnOptions};
use regex::Regex;
use shared_types::GameId;
use std::time::Duration;

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[component]
pub fn LiveTimer(side: Signal<Color>) -> impl IntoView {
    let timer_signals = expect_context::<TimerSignal>();
    let game_state = expect_context::<GameStateSignal>();
    let sounds = expect_context::<SoundsSignal>();
    let auth_context = expect_context::<AuthContext>();
    let user_id = Signal::derive(move || match untrack(auth_context.user) {
        Some(Ok(Some(user))) => Some(user.id),
        _ => None,
    });

    let user_color = game_state.user_color_as_signal(user_id.into());
    let in_progress = create_read_slice(game_state.signal, |gs| {
        gs.game_response
            .as_ref()
            .map_or(false, |gr| gr.game_status == GameStatus::InProgress)
    });

    let time_is_zero = Memo::new(move |_| {
        let timer = timer_signals.signal.get();
        let time_left = match side() {
            Color::White => timer.white_time_left.unwrap_or_default(),
            Color::Black => timer.black_time_left.unwrap_or_default(),
        };
        time_left == Duration::from_secs(0)
    });
    let user_needs_warning: Memo<bool> = Memo::new(move |_| {
        if let Some(color) = user_color() {
            let trigger_at = timer_signals.low_time.trigger_at.get();
            if color != side() || trigger_at.is_none() {
                return false;
            }

            let trigger_at = trigger_at.unwrap_or_default();

            let timer = timer_signals.signal.get();
            let time_left = match color {
                Color::White => timer.white_time_left.unwrap_or_default(),
                Color::Black => timer.black_time_left.unwrap_or_default(),
            };

            return !timer.finished
                && !timer_signals.low_time.issued.get()
                && time_left < trigger_at;
        }
        false
    });

    let should_refresh_warning: Memo<bool> = Memo::new(move |_| {
        if let Some(color) = user_color() {
            let refresh_at = timer_signals.low_time.refresh_at.get();
            if color != side() || refresh_at.is_none() {
                return false;
            }

            let refresh_at = refresh_at.unwrap_or(Duration::MAX);
            let timer = timer_signals.signal.get();
            let time_left = match color {
                Color::White => timer.white_time_left.unwrap_or_default(),
                Color::Black => timer.black_time_left.unwrap_or_default(),
            };

            return !timer.finished
                && timer_signals.low_time.issued.get()
                && time_left > refresh_at;
        }
        false
    });
    let time_is_red = Memo::new(move |_| {
        if time_is_zero() {
            String::from("bg-ladybug-red")
        } else {
            String::new()
        }
    });
    let tick_rate = Duration::from_millis(100);
    let Pausable { pause, resume, .. } = use_interval_fn_with_options(
        move || {
            let timer = timer_signals.signal.get();
            if !timer.finished {
                batch(move || {
                    timer_signals.signal.update(|timer| {
                        if timer.turn % 2 == 0 {
                            timer.white_time_left = timer.white_time_left.map(|t| {
                                t.checked_sub(tick_rate).unwrap_or(Duration::from_secs(0))
                            });
                        } else {
                            timer.black_time_left = timer.black_time_left.map(|t| {
                                t.checked_sub(tick_rate).unwrap_or(Duration::from_secs(0))
                            });
                        }
                    });
                });
            }
        },
        100,
        UseIntervalFnOptions::default().immediate(false),
    );

    create_effect(move |_| {
        let timer = timer_signals.signal.get();
        if in_progress() {
            if (side() == Color::White) == (timer.turn % 2 == 0) && !timer.finished {
                resume();
            } else {
                pause();
            }
        }
    });

    create_effect(move |_| {
        // When time runs out declare winner and style timer that ran out
        if time_is_zero() && !timer_signals.signal.get().finished {
            let api = ApiRequests::new();
            let router = expect_context::<RouterContext>();
            if let Some(caps) = NANOID.captures(&router.pathname().get_untracked()) {
                if let Some(nanoid) = caps.name("nanoid") {
                    let game_id = GameId(nanoid.as_str().to_string());
                    api.game_check_time(&game_id);
                }
            }
        }
    });

    create_effect(move |_| {
        if user_needs_warning() {
            sounds.play_sound(SoundType::LowTime);
            timer_signals.low_time.issued.set(true);
        }
    });
    create_effect(move |_| {
        if should_refresh_warning() {
            timer_signals.low_time.issued.set(false);
        }
    });

    view! {
        <div class=move || {
            format!(
                "flex resize h-full select-none items-center justify-center text-xl md:text-2xl lg:text-4xl {}",
                time_is_red(),
            )
        }>
            {move || {
                let timer = timer_signals.signal.get();
                let time_left = match side() {
                    Color::White => timer.white_time_left.unwrap_or_default(),
                    Color::Black => timer.black_time_left.unwrap_or_default(),
                };
                timer.time_mode.time_remaining(time_left)
            }}

        </div>
    }
}
