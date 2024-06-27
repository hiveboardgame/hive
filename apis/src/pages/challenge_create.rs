use crate::common::TimeSignals;
use crate::{
    common::ChallengeAction,
    components::{
        atoms::{
            create_challenge_button::CreateChallengeButton, input_slider::InputSlider,
            select_options::SelectOption,
        },
        organisms::time_select::TimeSelect,
    },
    providers::{ApiRequests, AuthContext},
};
use hive_lib::{ColorChoice, GameType};
use leptos::ev::Event;
use leptos::*;
use shared_types::{
    ChallengeDetails, ChallengeVisibility, CorrespondenceMode, GameSpeed, TimeMode,
};
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub struct ChallengeParams {
    pub rated: RwSignal<bool>,
    pub game_type: RwSignal<GameType>,
    pub visibility: RwSignal<ChallengeVisibility>,
    pub opponent: RwSignal<Option<String>>,
    pub color_choice: RwSignal<ColorChoice>,
    pub time_mode: RwSignal<TimeMode>,
    pub time_base: StoredValue<Option<i32>>,
    pub time_increment: StoredValue<Option<i32>>,
    pub band_upper: RwSignal<Option<i32>>,
    pub band_lower: RwSignal<Option<i32>>,
}

#[component]
pub fn ChallengeCreate(
    close: Callback<()>,
    #[prop(optional)] opponent: Option<String>,
) -> impl IntoView {
    let opponent = store_value(opponent);
    let params = ChallengeParams {
        rated: RwSignal::new(true),
        game_type: RwSignal::new(GameType::MLP),
        visibility: RwSignal::new(ChallengeVisibility::Public),
        opponent: RwSignal::new(opponent()),
        color_choice: RwSignal::new(ColorChoice::Random),
        time_mode: RwSignal::new(TimeMode::RealTime),
        time_base: store_value(None),
        time_increment: store_value(None),
        band_upper: RwSignal::new(None),
        band_lower: RwSignal::new(None),
    };
    let is_rated = move |b| {
        params.rated.update(|v| *v = b);
        if b {
            params.game_type.update(|v| *v = GameType::MLP)
        };
    };
    let has_expansions = move |game_type| {
        params.game_type.update(|v| *v = game_type);
        if game_type == GameType::Base {
            params.rated.update(|v| *v = false)
        };
    };
    let challenge_visibility = move |visibility| params.visibility.update(|v| *v = visibility);
    let band_upper = RwSignal::new(550_i32);
    let band_lower = RwSignal::new(-550_i32);

    let time_signals = TimeSignals::default();
    let create_challenge = Callback::new(move |color_choice| {
        params.color_choice.update_untracked(|p| *p = color_choice);
        let api = ApiRequests::new();
        params
            .band_upper
            .update_untracked(|v| *v = Some(band_upper.get_untracked()));
        params
            .band_lower
            .update_untracked(|v| *v = Some(band_lower.get_untracked()));
        params
            .time_mode
            .update_untracked(|v| *v = time_signals.time_control.get_untracked());
        match (params.time_mode)() {
            TimeMode::Untimed => {
                params.time_base.update_value(|v| *v = None);
                params.time_increment.update_value(|v| *v = None);
            }
            TimeMode::RealTime => {
                params
                    .time_base
                    .update_value(|v| *v = Some(time_signals.total_seconds.get_untracked()));
                params
                    .time_increment
                    .update_value(|v| *v = Some(time_signals.sec_per_move.get_untracked()));
            }
            TimeMode::Correspondence => {
                match time_signals.corr_mode.get_untracked() {
                    CorrespondenceMode::DaysPerMove => {
                        params.time_increment.update_value(|v| {
                            *v = Some(time_signals.corr_days.get_untracked() * 86400)
                        });
                        params.time_base.update_value(|v| *v = None);
                    }
                    CorrespondenceMode::TotalTimeEach => {
                        params.time_increment.update_value(|v| *v = None);
                        params.time_base.update_value(|v| {
                            *v = Some(time_signals.corr_days.get_untracked() * 86400)
                        });
                    }
                };
            }
        };
        let auth_context = expect_context::<AuthContext>();
        let account = move || match (auth_context.user)() {
            Some(Ok(Some(account))) => Some(account),
            _ => None,
        };
        let upper_rating = move || {
            if let (Some(band_upper), Some(account)) =
                (params.band_upper.get_untracked(), account())
            {
                if band_upper > 500 || opponent().is_some() {
                    return None;
                };
                // TODO: Make rating update in realtime, currently it becomes stale
                let rating = account
                    .user
                    .rating_for_speed(&GameSpeed::from_base_increment(
                        (params.time_base)(),
                        (params.time_increment)(),
                    ));
                Some((rating as i32).saturating_add(band_upper))
            } else {
                None
            }
        };
        let lower_rating = move || {
            if let (Some(band_lower), Some(account)) =
                (params.band_lower.get_untracked(), account())
            {
                if band_lower < -500 || opponent().is_some() {
                    return None;
                };
                let rating = account
                    .user
                    .rating_for_speed(&GameSpeed::from_base_increment(
                        (params.time_base)(),
                        (params.time_increment)(),
                    ));
                Some((rating as i32).saturating_add(band_lower))
            } else {
                None
            }
        };
        let details = ChallengeDetails {
            rated: params.rated.get_untracked(),
            game_type: params.game_type.get_untracked(),
            visibility: if opponent().is_none() {
                params.visibility.get_untracked()
            } else {
                ChallengeVisibility::Direct
            },
            opponent: params.opponent.get_untracked(),
            color_choice: params.color_choice.get_untracked(),
            time_mode: params.time_mode.get_untracked(),
            time_base: (params.time_base)(),
            time_increment: (params.time_increment)(),
            band_upper: upper_rating(),
            band_lower: lower_rating(),
        };
        let challenge_action = ChallengeAction::Create(details);
        api.challenge(challenge_action);
        close(());
    });

    let buttons_style =
        "my-1 p-1 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded";
    let disable_rated = move || {
        if (params.game_type)() == GameType::Base
            || time_signals.time_control.get() == TimeMode::Untimed
        {
            return true;
        }
        false
    };

    let active_color = move |b| {
        if b {
            "bg-button-dawn dark:bg-button-twilight"
        } else {
            "bg-odd-light dark:bg-gray-700"
        }
    };
    let rating_string = move || {
        format!(
            "{}/+{}",
            if band_lower() < -500 {
                "-∞".to_owned()
            } else {
                band_lower.get().to_string()
            },
            if band_upper() > 500 {
                "∞".to_owned()
            } else {
                band_upper().to_string()
            }
        )
    };

    let on_change: Callback<Event, ()> = Callback::from(move |ev: Event| {
        if let Ok(new_value) = TimeMode::from_str(&event_target_value(&ev)) {
            params
                .visibility
                .update(|v| *v = ChallengeVisibility::Public);
            params.game_type.update(|v| *v = GameType::MLP);
            time_signals.corr_days.update_untracked(|v| *v = 2);
            if new_value == TimeMode::Untimed {
                params.rated.set(false);
            } else {
                params.rated.set(true);
            }
            time_signals.time_control.update(|v| *v = new_value);
        };
    });

    view! {
        <div class="flex flex-col items-center w-72 xs:m-2 xs:w-80 sm:w-96">
            <div class=move || {
                opponent().map_or("hidden", |_| "block")
            }>"Opponent: " {opponent()}</div>
            <TimeSelect title=" Create a game:" time_signals on_change>
                <SelectOption value=time_signals.time_control is="Real Time"/>
                <SelectOption value=time_signals.time_control is="Correspondence"/>
                <SelectOption value=time_signals.time_control is="Untimed"/>
            </TimeSelect>
            <div class="flex justify-center">
                <button
                    prop:disabled=disable_rated
                    class=move || {
                        format!(
                            "disabled:opacity-25 disabled:cursor-not-allowed {buttons_style} {}",
                            active_color((params.rated)()),
                        )
                    }

                    on:click=move |_| is_rated(true)
                >
                    Rated
                </button>
                <button
                    class=move || { format!("{buttons_style} {}", active_color(!(params.rated)())) }
                    on:click=move |_| is_rated(false)
                >
                    Casual
                </button>
            </div>
            <div class="flex justify-center">
                <button
                    class=move || {
                        format!(
                            "{buttons_style} {}",
                            active_color((params.game_type)() == GameType::MLP),
                        )
                    }

                    on:click=move |_| has_expansions(GameType::MLP)
                >
                    PLM
                </button>
                <button
                    class=move || {
                        format!(
                            "{buttons_style} {}",
                            active_color((params.game_type)() == GameType::Base),
                        )
                    }

                    on:click=move |_| has_expansions(GameType::Base)
                >
                    Base
                </button>
            </div>
            <div class=move || {
                format!("{} justify-center", opponent().map_or("flex", |_| "hidden"))
            }>
                <button
                    class=move || {
                        format!(
                            "{buttons_style} {}",
                            active_color((params.visibility)() == ChallengeVisibility::Public),
                        )
                    }

                    on:click=move |_| challenge_visibility(ChallengeVisibility::Public)
                >
                    Public
                </button>
                <button
                    class=move || {
                        format!(
                            "{buttons_style} {}",
                            active_color((params.visibility)() == ChallengeVisibility::Private),
                        )
                    }

                    on:click=move |_| challenge_visibility(ChallengeVisibility::Private)
                >
                    Private
                </button>
            </div>
            <div class=move || {
                format!(
                    "{} flex-col items-center",
                    if opponent().is_some() { "hidden" } else { "flex" },
                )
            }>
                <p class="flex justify-center">Rating range</p>
                <div class="flex justify-center w-24">{rating_string}</div>
                <div class="flex">
                    <div class="flex gap-1 mx-1">
                        <label class="flex items-center">
                            <InputSlider
                                signal_to_update=band_lower
                                name="above"
                                min=-550
                                max=0
                                step=50
                            />
                        </label>
                        <label class="flex items-center">
                            <InputSlider
                                signal_to_update=band_upper
                                name="below"
                                min=0
                                max=550
                                step=50
                            />
                        </label>
                    </div>
                </div>
            </div>
            <div class="flex justify-center items-baseline">
                <CreateChallengeButton
                    color_choice=store_value(ColorChoice::White)
                    create_challenge
                />
                <CreateChallengeButton
                    color_choice=store_value(ColorChoice::Random)
                    create_challenge
                />
                <CreateChallengeButton
                    color_choice=store_value(ColorChoice::Black)
                    create_challenge
                />
            </div>
        </div>
    }
}
