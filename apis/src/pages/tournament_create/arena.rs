use super::common::{validate_configuration, CommonDraft, CreationDetails, CreationShell};
use crate::{
    components::{
        atoms::{input_slider::InputSlider, rating::icon_for_speed},
        molecules::panel::Panel,
    },
    i18n::*,
};
use leptos::prelude::*;
use leptos_i18n::I18nContext;
use leptos_icons::*;
use shared_types::{
    tournament::{
        arena::Config as ArenaConfig,
        BotAdmission,
        Clock,
        Config,
        Format,
        FormatConfig,
        RealtimeClock,
    },
    GameSpeed,
};
use std::num::NonZeroU32;

const ARENA_TIME_CONTROLS: [(i32, i32); 4] = [(1, 2), (3, 3), (5, 4), (10, 10)];

#[derive(Clone, Copy)]
struct ArenaDraft {
    duration_minutes: RwSignal<i32>,
    clock: RwSignal<RealtimeClock>,
}

impl ArenaDraft {
    fn new() -> Self {
        Self {
            duration_minutes: RwSignal::new(60),
            clock: RwSignal::new(RealtimeClock {
                base_seconds: NonZeroU32::new(600).unwrap(),
                increment_seconds: 10,
            }),
        }
    }

    fn configuration(self) -> Result<Config, String> {
        let duration = u32::try_from(self.duration_minutes.get())
            .ok()
            .and_then(|minutes| minutes.checked_mul(60))
            .and_then(NonZeroU32::new)
            .ok_or_else(|| String::from("Invalid Arena duration"))?;
        validate_configuration(Config {
            bot_admission: BotAdmission::HumansAndBots,
            format: FormatConfig::Arena(ArenaConfig::new(duration, self.clock.get())),
        })
    }
}

fn game_speed_text(i18n: I18nContext<Locale, I18nKeys>, speed: GameSpeed) -> String {
    match speed {
        GameSpeed::Bullet => t_string!(i18n, game.speeds.bullet),
        GameSpeed::Blitz => t_string!(i18n, game.speeds.blitz),
        GameSpeed::Rapid => t_string!(i18n, game.speeds.rapid),
        GameSpeed::Classic => t_string!(i18n, game.speeds.classic),
        GameSpeed::Correspondence => t_string!(i18n, game.speeds.correspondence),
        GameSpeed::Untimed => t_string!(i18n, game.speeds.untimed),
        GameSpeed::Puzzle => t_string!(i18n, game.speeds.puzzle),
    }
    .to_string()
}

#[component]
pub(super) fn ArenaCreationForm() -> impl IntoView {
    let i18n = use_i18n();
    let common = CommonDraft::new(false, false);
    let draft = ArenaDraft::new();
    let clock = Signal::derive(move || Clock::Realtime(draft.clock.get()));
    view! {
        <CreationShell
            draft=common
            format=Signal::derive(|| Format::Arena)
            clock
            configuration=Callback::new(move |()| draft.configuration())
            field=Signal::derive(|| (None, 0))
            valid=Signal::derive(|| true)
            participant_summary=Signal::derive(String::new)
        >
            <CreationDetails draft=common>

                <div class="flex flex-col gap-1.5">
                    <div class="flex gap-3 justify-between items-center">
                        <span class="ui-field-label">
                            {t!(i18n, tournaments.creation.arena_length_minutes)}
                        </span>
                        <span class="font-bold text-gray-900 dark:text-gray-100">
                            // TODO: i18n once copy is approved.
                            {move || {
                                let minutes = draft.duration_minutes.get();
                                if minutes % 60 == 0 {
                                    format!("{} h", minutes / 60)
                                } else {
                                    format!("{} h {} min", minutes / 60, minutes % 60)
                                }
                            }}
                        </span>
                    </div>
                    <InputSlider
                        signal_to_update=draft.duration_minutes
                        name="Arena Duration"
                        min=60
                        max=300
                        step=15
                    />
                </div>
            </CreationDetails>
            <Panel
                title=move || t_string!(i18n, tournaments.creation.time_controls_title)
                body_class="space-y-4"
            >
                <span class="ui-field-label">
                    {t!(i18n, tournaments.creation.game_time_control)}
                </span>
                <div class="flex flex-wrap gap-2">
                    {ARENA_TIME_CONTROLS
                        .iter()
                        .map(|(minutes, seconds)| {
                            let (minutes, seconds) = (*minutes, *seconds);
                            let speed = GameSpeed::from_base_increment(
                                Some(minutes * 60),
                                Some(seconds),
                            );
                            let selected = move || {
                                draft.clock.get().base_seconds.get() == minutes as u32 * 60
                                    && draft.clock.get().increment_seconds == seconds as u32
                            };
                            view! {
                                <button
                                    type="button"
                                    title=move || {
                                        t_string!(
                                            i18n,
                                        tournaments.creation.arena_control_title,
                                        speed = game_speed_text(i18n, speed),
                                        minutes = minutes,
                                        seconds = seconds,
                                        )
                                            .to_string()
                                    }
                                    class=move || {
                                        if selected() {
                                            "flex gap-1 items-center ui-button ui-button-primary ui-button-sm"
                                        } else {
                                            "flex gap-1 items-center ui-button ui-button-secondary ui-button-sm"
                                        }
                                    }
                                    on:click=move |_| {
                                        draft
                                            .clock
                                            .set(RealtimeClock {
                                                base_seconds: NonZeroU32::new(minutes as u32 * 60).unwrap(),
                                                increment_seconds: seconds as u32,
                                            });
                                    }
                                >
                                    <Icon icon=icon_for_speed(speed) attr:class="size-4" />
                                    {format!("{minutes}+{seconds}")}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            </Panel>
        </CreationShell>
    }
}
