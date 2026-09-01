use crate::{
    common::FlashStyle,
    hooks::{
        flash_pulse::use_flash_pulse,
        time_warnings::{use_time_warnings, WarningClock},
    },
    providers::{flash::FlashSignal, SoundType, Sounds},
};
use leptos::prelude::*;
use leptos_use::{use_interval_fn_with_options, utils::Pausable, UseIntervalFnOptions};
use shared_types::{trigger_at, GameSpeed, Repeat, TimeMode, TimeWarning, WarningTrigger};
use std::time::Duration;

const TICK: Duration = Duration::from_millis(100);
// Enough lead-in to hear the cue land at its real speed, short enough that a
// whole rehearsal is over in seconds rather than minutes.
const RUN_UP: Duration = Duration::from_secs(3);
// 2s of clock per 100ms tick: fast enough to skip a long middlegame, slow enough
// that the clock is visibly winding down rather than teleporting.
const FAST_STEP: Duration = Duration::from_secs(2);

const CONTROLS: [(u32, u32); 6] = [(60, 0), (180, 0), (180, 2), (300, 5), (600, 10), (900, 10)];

fn clock_text(left: Duration) -> String {
    let seconds = left.as_secs();
    if seconds >= 60 {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    } else {
        format!("{seconds}s")
    }
}

fn control_label(base: u32, increment: u32) -> String {
    format!("{} + {}", base / 60, increment)
}

#[component]
pub fn WarningRehearsal(stages: Signal<Vec<TimeWarning>>) -> impl IntoView {
    let flash = expect_context::<FlashSignal>();
    let sounds = expect_context::<Sounds>();

    let control = RwSignal::new((300_u32, 5_u32));
    let base = Signal::derive(move || Some(Duration::from_secs(u64::from(control.get().0))));
    let increment = Signal::derive(move || Some(Duration::from_secs(u64::from(control.get().1))));
    let remaining = RwSignal::new(None::<Duration>);
    let turn = RwSignal::new(0_usize);
    let hurrying = RwSignal::new(false);

    let plotted = Signal::derive(move || {
        let base = base.get();
        let increment = increment.get();
        let mut rows: Vec<(TimeWarning, Option<Duration>)> = stages
            .get()
            .into_iter()
            .map(|stage| {
                let at = trigger_at(&stage, TimeMode::RealTime, base, increment);
                (stage, at)
            })
            .collect();
        // Firing order: the warning with the most time left happens first.
        rows.sort_by_key(|(_, at)| std::cmp::Reverse(at.unwrap_or(Duration::ZERO)));
        rows
    });

    let Pausable { pause, resume, .. } = use_interval_fn_with_options(
        move || {
            let fires: Vec<Duration> = plotted
                .get_untracked()
                .into_iter()
                .flat_map(|(_, at)| at)
                .collect();
            remaining.update(|clock| {
                let Some(current) = *clock else { return };
                // The next warning we will hit is the highest one still below us.
                let target = fires.iter().filter(|at| **at < current).max().copied();
                let winding = target.is_some_and(|at| current > at + RUN_UP);
                hurrying.set(winding);
                let next = current.saturating_sub(if winding { FAST_STEP } else { TICK });
                // Never overshoot the lead-in, or the warning gets skipped.
                let next = match target {
                    Some(at) if winding && next < at + RUN_UP => at + RUN_UP,
                    _ => next,
                };
                *clock = if next.is_zero() { None } else { Some(next) };
            });
        },
        100,
        UseIntervalFnOptions::default().immediate(false),
    );

    Effect::watch(
        move || remaining.get().is_some(),
        move |running, _, _| {
            if *running {
                resume();
            } else {
                pause();
            }
        },
        false,
    );

    use_time_warnings(
        WarningClock {
            time_left: remaining.into(),
            time_base: base,
            increment,
            time_mode: Signal::derive(|| TimeMode::RealTime),
            speed: Signal::derive(move || {
                let (base, step) = control.get();
                Some(GameSpeed::from_base_increment(
                    Some(base as i32),
                    Some(step as i32),
                ))
            }),
            turn: turn.into(),
            active: Signal::derive(move || remaining.get().is_some()),
            stages,
        },
        {
            let sounds = sounds.clone();
            Callback::new(move |stage: TimeWarning| {
                if stage.sound {
                    sounds.play_sound(if stage.repeat == Repeat::EverySecond {
                        SoundType::Tick
                    } else {
                        match stage.at {
                            WarningTrigger::Proportional => SoundType::LowTime,
                            WarningTrigger::Remaining(_) => SoundType::Critical,
                        }
                    });
                }
                if stage.flash {
                    flash.fire(stage.flash_styles.clone());
                }
            })
        },
    );

    let start = move |_| {
        let highest = plotted
            .get_untracked()
            .into_iter()
            .flat_map(|(_, at)| at)
            .max();
        turn.set(0);
        let _ = highest;
        hurrying.set(false);
        remaining.set(base.get_untracked());
    };

    let map = move || {
        let base = base.get();
        plotted
            .get()
            .into_iter()
            .map(|(stage, at)| {
                let (text, muted) = match (at, base) {
                    (Some(at), Some(base)) => (
                        {
                            let _ = base;
                            format!(
                                "{} — fires with {} left",
                                match stage.at {
                                    WarningTrigger::Proportional =>
                                        "Scaled to the game".to_string(),
                                    WarningTrigger::Remaining(seconds) =>
                                        format!("{seconds} seconds left"),
                                },
                                clock_text(at),
                            )
                        },
                        false,
                    ),
                    _ => (
                        format!(
                            "{} — never fires at this time control, it is at least half the clock",
                            match stage.at {
                                WarningTrigger::Proportional => "Scaled to the game".to_string(),
                                WarningTrigger::Remaining(seconds) =>
                                    format!("{seconds} seconds left"),
                            },
                        ),
                        true,
                    ),
                };
                view! {
                    <li class=if muted {
                        "ui-field-helper line-through opacity-60"
                    } else {
                        "ui-field-helper"
                    }>{text}</li>
                }
            })
            .collect_view()
    };

    let mock_timer_flashing = use_flash_pulse(FlashStyle::Timer);
    let mock_board_flashing = use_flash_pulse(FlashStyle::Board);
    let overlay_flashing = use_flash_pulse(FlashStyle::Screen);

    view! {
        <div class="flex flex-col gap-3">
            <p class="ui-field-label">"Try it out"</p>
            <select
                class="ui-field-select"
                prop:value=move || {
                    let (base, increment) = control.get();
                    control_label(base, increment)
                }
                on:change=move |ev| {
                    let picked = event_target_value(&ev);
                    if let Some(found) = CONTROLS
                        .iter()
                        .find(|(base, increment)| control_label(*base, *increment) == picked)
                    {
                        control.set(*found);
                    }
                }
            >
                {CONTROLS
                    .iter()
                    .map(|(base, increment)| {
                        let label = control_label(*base, *increment);
                        let value = label.clone();
                        view! {
                            <option value=value selected=*base == 300 && *increment == 5>
                                {label}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>

            <ul class="flex flex-col gap-1">{map}</ul>

            <div class="flex flex-wrap gap-2 items-center">
                <button
                    type="button"
                    class="ui-choice ui-choice-sm ui-choice-inactive"
                    on:click=start
                >
                    "Run"
                </button>
                <button
                    type="button"
                    class="ui-choice ui-choice-sm ui-choice-inactive"
                    disabled=move || remaining.get().is_none()
                    title="Give back the increment and hand the turn back to you, the way a real move does"
                    on:click=move |_| {
                        let step = increment.get_untracked().unwrap_or_default();
                        turn.update(|value| *value += 2);
                        remaining
                            .update(|clock| {
                                if let Some(current) = clock {
                                    *current += step;
                                }
                            });
                    }
                >
                    "Simulate move"
                </button>
                <span class="font-mono tabular-nums ui-field-helper">
                    {move || match remaining.get() {
                        Some(left) => format!("{:.1}s", left.as_secs_f32()),
                        None => "idle".to_string(),
                    }}
                </span>
                <Show when=move || hurrying.get()>
                    <span class="ui-field-helper">"winding forward to the next warning"</span>
                </Show>
            </div>

            <div class="flex gap-3 items-center">
                <div class=move || {
                    format!(
                        "flex items-center justify-center rounded-md border border-black/10 px-3 py-2 font-mono text-lg dark:border-white/10 {}",
                        if mock_timer_flashing.get() { "warning-flash-timer" } else { "" },
                    )
                }>
                    {move || match remaining.get() {
                        Some(left) => format!("{:.1}", left.as_secs_f32()),
                        None => base.get().map_or_else(|| "0:00".to_string(), clock_text),
                    }}
                </div>
                <div class=move || {
                    format!(
                        "flex h-14 grow items-center justify-center rounded-md border border-black/10 bg-board-dawn text-xs text-black/40 dark:border-white/10 dark:bg-board-twilight dark:text-white/40 {}",
                        if mock_board_flashing.get() { "warning-flash-board" } else { "" },
                    )
                }>"board"</div>
            </div>
            <p class="ui-field-helper">
                "Run starts on the clock you picked, skips the quiet stretches, and plays each warning at real speed. Simulate move gives back the increment and hands you the turn, so you can see which warnings arm again. The clock and board above stand in for the real ones."
            </p>

            <Show when=move || overlay_flashing.get()>
                <div class="fixed inset-0 z-50 pointer-events-none warning-flash-overlay"></div>
            </Show>
        </div>
    }
}
