use crate::common::TimeSignals;
use crate::{
    common::ChallengeAction,
    components::{
        atoms::{
            create_challenge_button::CreateChallengeButton, input_slider::InputSlider,
            simple_switch::SimpleSwitch,
        },
        organisms::time_select::TimeSelect,
    },
    providers::{ApiRequests, AuthContext},
};
use hive_lib::{ColorChoice, GameType};
use leptos::*;
use shared_types::{
    ChallengeDetails, ChallengeVisibility, CorrespondenceMode, GameSpeed, TimeMode,
};
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub struct ChallengeParams {
    pub rated: RwSignal<bool>,
    pub with_expansions: RwSignal<bool>,
    pub is_public: RwSignal<bool>,
    pub opponent: RwSignal<Option<String>>,
    pub color_choice: RwSignal<ColorChoice>,
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
        with_expansions: RwSignal::new(true),
        is_public: RwSignal::new(true),
        opponent: RwSignal::new(opponent()),
        color_choice: RwSignal::new(ColorChoice::Random),
        time_base: store_value(None),
        time_increment: store_value(None),
        band_upper: RwSignal::new(None),
        band_lower: RwSignal::new(None),
    };
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
        match time_signals.time_mode.get() {
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
            game_type: if params.with_expansions.get_untracked() {
                GameType::MLP
            } else {
                GameType::Base
            },
            visibility: if opponent().is_none() {
                if params.is_public.get_untracked() {
                    ChallengeVisibility::Public
                } else {
                    ChallengeVisibility::Private
                }
            } else {
                ChallengeVisibility::Direct
            },
            opponent: params.opponent.get_untracked(),
            color_choice: params.color_choice.get_untracked(),
            time_mode: time_signals.time_mode.get_untracked(),
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
            time_signals.corr_days.update_untracked(|v| *v = 2);
            if new_value == TimeMode::Untimed {
                params.rated.set(false);
            }
            time_signals.time_mode.update(|v| *v = new_value);
        };
    });
    let allowed_values = vec![
        TimeMode::RealTime,
        TimeMode::Correspondence,
        TimeMode::Untimed,
    ];
    let make_unrated = Callback::new(move |()| {
        if !params.with_expansions.get() {
            params.rated.set(false)
        }
    });
    let add_expansions = Callback::new(move |()| {
        if params.rated.get() {
            params.with_expansions.set(true)
        }
    });
    let untimed_no_rated =
        Signal::derive(move || time_signals.time_mode.get() == TimeMode::Untimed);

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
            <div class="flex gap-1 p-1">
                Casual
                <SimpleSwitch
                    checked=params.rated
                    optional_action=add_expansions
                    disabled=untimed_no_rated
                /> Rated
            </div>
            <div class="flex gap-1 p-1">
                Base <SimpleSwitch checked=params.with_expansions optional_action=make_unrated/> MLP
            </div>

            <Show when=move || opponent().is_none()>
                <div class="flex gap-1 p-1">
                    Private <SimpleSwitch checked=params.is_public/> Public
                </div>
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
