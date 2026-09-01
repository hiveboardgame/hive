use crate::{
    common::FlashStyle,
    components::{molecules::panel::Panel, organisms::warning_rehearsal::WarningRehearsal},
    functions::accounts::edit::EditTimeWarnings,
    providers::{flash::FlashSignal, AuthContext, ClockWarningDraft, SoundType, Sounds},
};
use leptos::{either::Either, prelude::*};
use leptos_use::use_debounce_fn;
use shared_types::{
    default_time_warnings,
    GameSpeed,
    Repeat,
    TimeWarning,
    WarningTrigger,
    MAX_TIME_WARNINGS,
};

const PRESETS: [u32; 5] = [60, 30, 15, 10, 5];

fn label_for(trigger: WarningTrigger) -> String {
    match trigger {
        WarningTrigger::Proportional => "Scaled to the game".to_string(),
        WarningTrigger::Remaining(seconds) => format!("{seconds} seconds left"),
    }
}

fn value_for(trigger: WarningTrigger) -> String {
    match trigger {
        WarningTrigger::Proportional => "Proportional".to_string(),
        WarningTrigger::Remaining(seconds) => seconds.to_string(),
    }
}

fn trigger_from(value: &str) -> WarningTrigger {
    value
        .parse::<u32>()
        .map_or(WarningTrigger::Proportional, WarningTrigger::Remaining)
}

fn cue_summary(stage: &TimeWarning) -> String {
    let mut cues = Vec::new();
    if stage.sound {
        cues.push("Sound");
    }
    if stage.flash {
        cues.push("Flash");
    }
    if cues.is_empty() {
        "Silent".to_string()
    } else {
        cues.join(" + ")
    }
}

fn speed_summary(stage: &TimeWarning) -> String {
    if stage.speeds.is_empty() {
        return "No game speeds".to_string();
    }
    if stage.speeds.len() == GameSpeed::real_time_speeds().len() {
        return "All speeds".to_string();
    }
    stage
        .speeds
        .iter()
        .map(GameSpeed::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn unused_preset(stages: &[TimeWarning]) -> WarningTrigger {
    let taken = |trigger: WarningTrigger| stages.iter().any(|stage| stage.at == trigger);
    if !taken(WarningTrigger::Proportional) {
        return WarningTrigger::Proportional;
    }
    PRESETS
        .iter()
        .map(|seconds| WarningTrigger::Remaining(*seconds))
        .find(|trigger| !taken(*trigger))
        .unwrap_or(WarningTrigger::Remaining(5))
}

#[component]
pub fn ClockWarnings() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();

    let sounds = expect_context::<Sounds>();
    // Shadow the app-wide pulse: nothing on this page renders a real flash, so
    // previews drive the rehearsal mocks instead of firing into the void.
    provide_context(FlashSignal {
        pulse: RwSignal::new(None),
    });
    let flash = expect_context::<FlashSignal>();
    let action = ServerAction::<EditTimeWarnings>::new();

    let stored = Signal::derive(move || {
        auth_context
            .user
            .with(|user| user.as_ref().map(|account| account.time_warnings.clone()))
            .unwrap_or_else(default_time_warnings)
    });
    let ClockWarningDraft {
        stages: draft,
        editing,
        seeded,
    } = expect_context();
    if draft.with_untracked(Vec::is_empty) {
        draft.set(stored.get_untracked());
    }
    Effect::watch(
        move || stored.get(),
        move |latest, _, _| {
            // The account resolves after first paint, so adopt it once. Refetches
            // after that (a window refocus, say) must not wipe unsaved edits.
            if !seeded.get_untracked() {
                seeded.set(true);
                draft.set(latest.clone());
            }
        },
        false,
    );
    let action_value = action.value();
    Effect::watch(
        action.version(),
        move |_, _, _| {
            if action_value
                .get_untracked()
                .is_some_and(|result| result.is_ok())
            {
                auth_context.refresh_account();
            }
        },
        false,
    );

    let autosave = use_debounce_fn(
        move || {
            action.dispatch(EditTimeWarnings {
                warnings: draft.get_untracked(),
            });
        },
        600.0,
    );
    Effect::watch(
        move || draft.get(),
        move |latest, _, _| {
            // Not gated on `seeded`: the account is usually already resolved at mount,
            // so that flag may never flip. Comparing against `stored` is enough —
            // seeding writes an equal value and so saves nothing.
            if *latest != stored.get_untracked() {
                autosave();
            }
        },
        false,
    );
    let can_add = Signal::derive(move || draft.with(|stages| stages.len() < MAX_TIME_WARNINGS));

    let preview = Callback::new(move |stage: TimeWarning| {
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
    });

    let rows = move || {
        draft
            .get()
            .into_iter()
            .enumerate()
            .map(|(index, stage)| {
                let open = editing.get() == Some(index);
                let cue = move |pick: fn(&mut TimeWarning) -> &mut bool| {
                    draft.update(|stages| {
                        if let Some(target) = stages.get_mut(index) {
                            let flag = pick(target);
                            *flag = !*flag;
                        }
                    });
                };
                let summary = stage.clone();
                let testable = stage.clone();
                let shows_flash = stage.flash;
                if !open {
                    return Either::Left(
                        view! {
                            <button
                                type="button"
                                class="flex flex-wrap gap-y-1 gap-x-3 items-center w-full text-left ui-setting-group dark:hover:bg-surface-muted hover:bg-odd-light"
                                on:click=move |_| editing.set(Some(index))
                            >
                                <span class="font-semibold">{label_for(summary.at)}</span>
                                <span class="ui-field-helper">{cue_summary(&summary)}</span>
                                <span class="ui-field-helper">{summary.repeat.label()}</span>
                                <span class="ui-field-helper">{speed_summary(&summary)}</span>
                                <span class="ml-auto ui-field-helper">"Edit"</span>
                            </button>
                        },
                    );
                }
                Either::Right(
                    view! {
                        <div class="flex flex-col gap-3 ui-setting-group">
                            <div class="flex flex-col gap-1">
                                <p class="ui-field-label">"Warn when"</p>
                                <select
                                    class="ui-field-select"
                                    on:change=move |ev| {
                                        let picked = trigger_from(&event_target_value(&ev));
                                        draft
                                            .update(|stages| {
                                                if let Some(target) = stages.get_mut(index) {
                                                    target.at = picked;
                                                }
                                            });
                                    }
                                >
                                    <option
                                        value=value_for(WarningTrigger::Proportional)
                                        selected=stage.at == WarningTrigger::Proportional
                                    >
                                        {label_for(WarningTrigger::Proportional)}
                                    </option>
                                    {PRESETS
                                        .iter()
                                        .map(|seconds| {
                                            let trigger = WarningTrigger::Remaining(*seconds);
                                            view! {
                                                <option
                                                    value=value_for(trigger)
                                                    selected=stage.at == trigger
                                                >
                                                    {label_for(trigger)}
                                                </option>
                                            }
                                        })
                                        .collect_view()}
                                </select>
                            </div>

                            <div class="flex flex-col gap-1">
                                <p class="ui-field-label">"How it warns"</p>
                                <div class="ui-choice-group">
                                    <button
                                        type="button"
                                        class="ui-choice ui-choice-sm"
                                        class:ui-choice-active=stage.sound
                                        class:ui-choice-inactive=!stage.sound
                                        on:click=move |_| cue(|stage| &mut stage.sound)
                                    >
                                        "Sound"
                                    </button>
                                    <button
                                        type="button"
                                        class="ui-choice ui-choice-sm"
                                        class:ui-choice-active=stage.flash
                                        class:ui-choice-inactive=!stage.flash
                                        on:click=move |_| cue(|stage| &mut stage.flash)
                                    >
                                        "Flash"
                                    </button>
                                </div>
                            </div>

                            <Show when=move || shows_flash>
                                <div class="flex flex-col gap-1">
                                    <p class="ui-field-label">"Where it flashes"</p>
                                    <div class="ui-choice-group">
                                        {FlashStyle::ALL
                                            .into_iter()
                                            .map(|style| {
                                                let active = stage.flash_styles.contains(&style);
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="ui-choice ui-choice-sm"
                                                        class:ui-choice-active=active
                                                        class:ui-choice-inactive=!active
                                                        on:click=move |_| {
                                                            draft
                                                                .update(|stages| {
                                                                    if let Some(target) = stages.get_mut(index) {
                                                                        if let Some(at) = target
                                                                            .flash_styles
                                                                            .iter()
                                                                            .position(|held| *held == style)
                                                                        {
                                                                            target.flash_styles.remove(at);
                                                                        } else {
                                                                            target.flash_styles.push(style);
                                                                        }
                                                                    }
                                                                });
                                                        }
                                                    >
                                                        {style.to_string()}
                                                    </button>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </div>
                            </Show>
                            <div class="flex flex-col gap-1">
                                <p class="ui-field-label">"How often"</p>
                                <div class="ui-choice-group">
                                    {Repeat::ALL
                                        .into_iter()
                                        .map(|repeat| {
                                            let active = stage.repeat == repeat;
                                            view! {
                                                <button
                                                    type="button"
                                                    class="ui-choice ui-choice-sm"
                                                    class:ui-choice-active=active
                                                    class:ui-choice-inactive=!active
                                                    on:click=move |_| {
                                                        draft
                                                            .update(|stages| {
                                                                if let Some(target) = stages.get_mut(index) {
                                                                    target.repeat = repeat;
                                                                }
                                                            });
                                                    }
                                                >
                                                    {repeat.label()}
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>

                            <div class="flex flex-col gap-1">
                                <p class="ui-field-label">"In which games"</p>
                                <div class="ui-choice-group">
                                    {GameSpeed::real_time_speeds()
                                        .into_iter()
                                        .map(|speed| {
                                            let active = stage.speeds.contains(&speed);
                                            view! {
                                                <button
                                                    type="button"
                                                    class="ui-choice ui-choice-sm"
                                                    class:ui-choice-active=active
                                                    class:ui-choice-inactive=!active
                                                    on:click=move |_| {
                                                        draft
                                                            .update(|stages| {
                                                                if let Some(target) = stages.get_mut(index) {
                                                                    if let Some(at) = target
                                                                        .speeds
                                                                        .iter()
                                                                        .position(|held| *held == speed)
                                                                    {
                                                                        target.speeds.remove(at);
                                                                    } else {
                                                                        target.speeds.push(speed);
                                                                    }
                                                                }
                                                            });
                                                    }
                                                >
                                                    {speed.to_string()}
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            </div>

                            <div class="flex flex-wrap gap-2 items-center pt-1">
                                <button
                                    type="button"
                                    class="ui-choice ui-choice-sm ui-choice-inactive"
                                    on:click=move |_| preview.run(testable.clone())
                                >
                                    "Test"
                                </button>
                                <button
                                    type="button"
                                    class="ui-choice ui-choice-sm ui-choice-inactive"
                                    on:click=move |_| {
                                        draft
                                            .update(|stages| {
                                                stages.remove(index);
                                            });
                                        editing.set(None);
                                    }
                                >
                                    "Remove"
                                </button>
                                <button
                                    type="button"
                                    class="ml-auto ui-choice ui-choice-sm ui-choice-active"
                                    on:click=move |_| editing.set(None)
                                >
                                    "Done"
                                </button>
                            </div>
                        </div>
                    },
                )
            })
            .collect_view()
    };

    view! {
        <Panel title="Clock warnings" body_class="space-y-4">
            <p class="ui-field-helper">
                "Warnings fire on your own clock in real time games. At most " {MAX_TIME_WARNINGS}
                ", so each one still means something."
            </p>

            <div class="flex flex-col gap-2">{rows}</div>

            <div class="flex flex-wrap gap-2 items-center">
                <Show when=move || can_add.get()>
                    <button
                        type="button"
                        class="ui-choice ui-choice-sm ui-choice-inactive"
                        on:click=move |_| {
                            draft
                                .update(|stages| {
                                    let at = unused_preset(stages);
                                    stages.push(TimeWarning::new(at));
                                });
                        }
                    >
                        "Add warning"
                    </button>
                </Show>
                <span class="ui-field-helper">
                    {move || {
                        if action.pending().get() { "Saving…" } else { "Saved automatically" }
                    }}
                </span>
            </div>

            <div class="pt-4 border-t border-black/5 dark:border-white/10">
                <WarningRehearsal stages=draft.into() />
            </div>
        </Panel>
    }
}
