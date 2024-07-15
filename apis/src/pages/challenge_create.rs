use crate::common::TimeSignals;
use crate::{
    common::ChallengeAction,
    components::{
        atoms::{create_challenge_button::CreateChallengeButton, input_slider::InputSlider},
        organisms::time_select::TimeSelect,
    },
    providers::{ApiRequests, AuthContext},
};
use hive_lib::{ColorChoice, GameType};
use leptix_primitives::radio_group::{RadioGroupItem, RadioGroupRoot};
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
    let (rated_value, set_rated_value) = create_signal("Rated".to_string());
    let band_upper = RwSignal::new(550_i32);
    let band_lower = RwSignal::new(-550_i32);
    let game_type = move || params.game_type.get();
    let time_control = move || params.time_mode.get();
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
        match time_control() {
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

    let time_change = Callback::from(move |s: String| {
        if let Ok(new_value) = TimeMode::from_str(&s) {
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
    let allowed_values = vec![
        TimeMode::RealTime,
        TimeMode::Correspondence,
        TimeMode::Untimed,
    ];
    let radio_style = "flex items-center my-1 p-1 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded data-[state=checked]:bg-button-dawn dark:data-[state=checked]:bg-button-twilight data-[state=unchecked]:bg-odd-light dark:data-[state=unchecked]:bg-gray-700 data-[state=unchecked]:bg-odd-light dark:data-[state=unchecked]:bg-gray-700";
    view! {
        <div class="flex flex-col items-center w-72 xs:m-2 xs:w-80 sm:w-96">
            <Show when=move || opponent().is_some()>
                <div class="block">"Opponent: " {opponent()}</div>
            </Show>
            <div class="flex flex-col items-center">
                <TimeSelect
                    title=" Create a game:"
                    time_signals
                    on_value_change=time_change
                    allowed_values
                />
            </div>
            <RadioGroupRoot
                required=true
                attr:class="flex justify-center"
                value=rated_value
                default_value="Rated"
                on_value_change=move |v| {
                    let is_rated = v == "Rated";
                    params.rated.update(|v| *v = is_rated);
                    if is_rated {
                        params.game_type.update(|v| *v = GameType::MLP);
                    }
                    set_rated_value(v)
                }
            >

                <RadioGroupItem
                    value="Rated"
                    attr:disabled=Signal::derive(move || {
                        if game_type() == GameType::Base || time_control() == TimeMode::Untimed {
                            set_rated_value("Casual".to_string());
                            return true;
                        }
                        false
                    })

                    attr:class=format!(
                        "disabled:opacity-25 disabled:cursor-not-allowed {radio_style}",
                    )
                >

                    "Rated"
                </RadioGroupItem>
                <RadioGroupItem value="Casual" attr:class=radio_style>
                    "Casual"
                </RadioGroupItem>
            </RadioGroupRoot>

            <RadioGroupRoot
                required=true
                attr:class="flex justify-center"
                default_value="MLP"
                on_value_change=move |v| {
                    let game_type = if v == "MLP" { GameType::MLP } else { GameType::Base };
                    params.game_type.update(|v| *v = game_type);
                    if game_type == GameType::Base {
                        params.rated.update(|v| *v = false)
                    }
                }
            >

                <RadioGroupItem value="MLP" attr:class=radio_style>
                    "MLP"
                </RadioGroupItem>
                <RadioGroupItem value="Base" attr:class=radio_style>
                    "Base"
                </RadioGroupItem>
            </RadioGroupRoot>

            <Show when=move || opponent().is_none()>
                <RadioGroupRoot
                    required=true
                    attr:class="flex justify-center"
                    default_value="Public"
                    on_value_change=move |value| {
                        params
                            .visibility
                            .update(|v| {
                                *v = if value == "Public" {
                                    ChallengeVisibility::Public
                                } else {
                                    ChallengeVisibility::Private
                                };
                            })
                    }
                >

                    <RadioGroupItem value="Public" attr:class=radio_style>
                        "Public"
                    </RadioGroupItem>
                    <RadioGroupItem value="Private" attr:class=radio_style>
                        "Private"
                    </RadioGroupItem>
                </RadioGroupRoot>
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
            </Show>
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
