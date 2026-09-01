use leptos::prelude::*;
use shared_types::{rearm_margin, trigger_at, GameSpeed, Repeat, TimeMode, TimeWarning};
use std::{collections::HashMap, time::Duration};

const COLLAPSE_WINDOW: Duration = Duration::from_secs(2);

#[derive(Clone, Copy)]
pub struct WarningClock {
    pub time_left: Signal<Option<Duration>>,
    pub time_base: Signal<Option<Duration>>,
    pub increment: Signal<Option<Duration>>,
    pub time_mode: Signal<TimeMode>,
    pub speed: Signal<Option<GameSpeed>>,
    pub turn: Signal<usize>,
    pub active: Signal<bool>,
    pub stages: Signal<Vec<TimeWarning>>,
}

#[derive(Default)]
struct StageLatch {
    armed: bool,
    last_second: Option<u64>,
}

// Two stages a second apart are a stutter, not two warnings, so neighbours
// collapse into the lower one with their cues merged.
fn resolved_stages(clock: &WarningClock) -> Vec<(Duration, TimeWarning)> {
    let mode = clock.time_mode.get();
    let base = clock.time_base.get();
    let increment = clock.increment.get();
    let speed = clock.speed.get();
    let mut stages: Vec<(Duration, TimeWarning)> = clock
        .stages
        .get()
        .into_iter()
        .filter(|stage| match speed {
            Some(speed) => stage.speeds.contains(&speed),
            None => true,
        })
        .filter_map(|stage| trigger_at(&stage, mode, base, increment).map(|at| (at, stage)))
        .collect();
    stages.sort_by_key(|(at, _)| *at);

    let mut collapsed: Vec<(Duration, TimeWarning)> = Vec::with_capacity(stages.len());
    for (at, stage) in stages {
        match collapsed.last_mut() {
            Some((kept, merged)) if at.saturating_sub(*kept) < COLLAPSE_WINDOW => {
                merged.sound |= stage.sound;
                merged.flash |= stage.flash;

                merged.repeat = merged.repeat.strongest(stage.repeat);
            }
            _ => collapsed.push((at, stage)),
        }
    }
    collapsed
}

pub fn use_time_warnings(clock: WarningClock, on_fire: Callback<TimeWarning>) {
    let latches: RwSignal<HashMap<Duration, StageLatch>> = RwSignal::new(HashMap::new());
    let last_turn: RwSignal<Option<usize>> = RwSignal::new(None);

    Effect::new(move |_| {
        let Some(left) = clock.time_left.get() else {
            return;
        };
        let stages = resolved_stages(&clock);
        if stages.is_empty() {
            return;
        }
        let increment = clock.increment.get();
        let active = clock.active.get();
        let turn = clock.turn.get();
        let turn_changed = last_turn.get_untracked() != Some(turn);
        last_turn.set(Some(turn));

        let mut firing = None;
        latches.update(|state| {
            state.retain(|threshold, _| stages.iter().any(|(at, _)| at == threshold));
            for (at, stage) in &stages {
                // A stage the clock is already below must start disarmed, or
                // reconnecting on low time fires everything at once.
                let latch = state.entry(*at).or_insert_with(|| StageLatch {
                    armed: left > *at,
                    last_second: None,
                });
                if left > *at {
                    latch.last_second = None;
                }
                if left > *at + rearm_margin(stage, increment)
                    || (turn_changed && active && stage.repeat == Repeat::EachTurn)
                {
                    latch.armed = true;
                }
                if !active || left >= *at {
                    continue;
                }
                let due = if stage.repeat == Repeat::EverySecond {
                    let second = left.as_secs();
                    let ticked = latch.last_second != Some(second);
                    latch.last_second = Some(second);
                    ticked
                } else if latch.armed {
                    latch.armed = false;
                    true
                } else {
                    false
                };
                if due && firing.is_none() {
                    firing = Some(stage.clone());
                }
            }
        });

        if let Some(stage) = firing {
            on_fire.run(stage);
        }
    });
}
