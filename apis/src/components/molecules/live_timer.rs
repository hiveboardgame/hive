use crate::providers::{
    game_state::GameStateSignal, timer::TimerSignal, ApiRequests, AuthContext, SoundType, Sounds,
};
use hive_lib::{Color, GameStatus};
use lazy_static::lazy_static;
use leptos::*;
use leptos_router::RouterContext;
use leptos_use::{
    use_interval_fn_with_options, utils::Pausable, watch_with_options, whenever_with_options,
    UseIntervalFnOptions, WatchOptions,
};
use regex::Regex;
use shared_types::GameId;
use std::time::Duration;

lazy_static! {
    static ref NANOID: Regex =
        Regex::new(r"/game/(?<nanoid>.*)").expect("This regex should compile");
}

#[component]
pub fn LiveTimer(side: Signal<Color>) -> impl IntoView {
    let game_state = expect_context::<GameStateSignal>();
    let sounds = expect_context::<Sounds>();
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
    let timer = expect_context::<TimerSignal>().signal;
    let tick_rate = Duration::from_millis(100);
    let Pausable { pause, resume, .. } = use_interval_fn_with_options(
        move || {
            timer.update(|t| {
                if t.turn % 2 == 0 {
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
        timer
            .with(|t| in_progress() && (side() == Color::White) == (t.turn % 2 == 0) && !t.finished)
    });
    let time_is_zero = Signal::derive(move || timer().time_left(side()).is_zero());
    let user_needs_warning = Signal::derive(move || {
        user_color().map_or(false, |color| {
            timer.with(|t| {
                t.warning_trigger().map_or(false, |trigger_at| {
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
        user_color().map_or(false, |color| {
            timer.with(|t| {
                t.warning_refresh().map_or(false, |refresh_at| {
                    if color == side() && !t.finished {
                        t.time_left(color) > refresh_at
                    } else {
                        false
                    }
                })
            })
        })
    });
    #[allow(unused)]
    watch_with_options(
        should_resume,
        move |v, _, _| {
            if *v {
                resume();
            } else {
                pause();
            }
        },
        //Has immediate = true, hence not unused
        WatchOptions::default().immediate(true),
    );

    #[allow(unused)]
    whenever_with_options(
        move || time_is_zero() && !timer().finished,
        move |v, _, _| {
            // When time runs out declare winner and style timer that ran out
            let api = ApiRequests::new();
            let router = expect_context::<RouterContext>();
            if let Some(caps) = NANOID.captures(&router.pathname().get_untracked()) {
                if let Some(nanoid) = caps.name("nanoid") {
                    let game_id = GameId(nanoid.as_str().to_string());
                    api.game_check_time(&game_id);
                }
            }
        },
        //Has immediate = true, hence not unused
        WatchOptions::default().immediate(true),
    );
    #[allow(unused)]
    whenever_with_options(
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
        //Has immediate = true, hence not unused
        WatchOptions::default().immediate(true),
    );

    view! {
        <div class=move || {
            format!(
                "flex resize h-full select-none items-center justify-center text-xl md:text-2xl lg:text-4xl {}",
                if time_is_zero() { "bg-ladybug-red" } else { "" },
            )
        }>
            {move || {
                let timer = timer();
                let time_left = timer.time_left(side());
                timer.time_mode.time_remaining(time_left)
            }}

        </div>
    }
}
