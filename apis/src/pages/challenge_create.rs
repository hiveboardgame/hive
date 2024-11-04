use crate::i18n::*;
use crate::providers::{AuthContext, ChallengeParams};
use crate::{
    common::ChallengeAction,
    components::{
        atoms::{
            create_challenge_button::CreateChallengeButton, input_slider::InputSlider,
            simple_switch::SimpleSwitch,
        },
        organisms::time_select::TimeSelect,
    },
    providers::ApiRequests,
};
use hive_lib::{ColorChoice, GameType};
use leptos::*;
use shared_types::{ChallengeDetails, ChallengeVisibility, GameSpeed, TimeMode};
use std::str::FromStr;

#[component]
pub fn ChallengeCreate(
    open: RwSignal<bool>,
    #[prop(optional)] opponent: Option<String>,
) -> impl IntoView {
    view! {
        <Show when=open>
            <ChallengeCreateInner opponent=opponent.clone() open />
        </Show>
    }
}

#[component]
fn ChallengeCreateInner(open: RwSignal<bool>, opponent: Option<String>) -> impl IntoView {
    let i18n = use_i18n();
    let params = expect_context::<ChallengeParams>();
    let opponent = store_value(opponent);
    let time_signals = params.time_signals;
    create_effect(move |_| {
        let opponent = opponent();
        if opponent.is_some() {
            params.opponent.update(|o| *o = opponent);
        }
    });
    let create_challenge = Callback::new(move |color_choice| {
        let api = ApiRequests::new();
        let auth_context = expect_context::<AuthContext>();
        let account = move || match (auth_context.user)() {
            Some(Ok(Some(account))) => Some(account),
            _ => None,
        };

        let upper_rating = move || {
            if let Some(account) = account() {
                let upper_slider = params.upper_slider.get();
                if upper_slider > 500 || opponent().is_some() {
                    return None;
                };
                // TODO: Make rating update in realtime, currently it becomes stale
                let rating = account
                    .user
                    .rating_for_speed(&GameSpeed::from_base_increment(
                        params.time_base.get(),
                        params.time_increment.get(),
                    ));
                Some((rating as i32).saturating_add(upper_slider))
            } else {
                None
            }
        };

        let lower_rating = move || {
            if let Some(account) = account() {
                let lower_slider = params.lower_slider.get();
                if lower_slider < -500 || opponent().is_some() {
                    return None;
                };
                // TODO: Make rating update in realtime, currently it becomes stale
                let rating = account
                    .user
                    .rating_for_speed(&GameSpeed::from_base_increment(
                        params.time_base.get(),
                        params.time_increment.get(),
                    ));
                Some((rating as i32).saturating_add(lower_slider))
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
            opponent: opponent(),
            color_choice,
            time_mode: time_signals.time_mode.get_untracked(),
            time_base: (params.time_base)(),
            time_increment: (params.time_increment)(),
            band_upper: upper_rating(),
            band_lower: lower_rating(),
        };
        let challenge_action = ChallengeAction::Create(details);
        api.challenge(challenge_action);
        open.set(false);
    });

    let rating_string = move || {
        let lower = params.lower_slider.get();
        let upper = params.upper_slider.get();
        format!(
            "{}/+{}",
            if lower < -500 {
                "-∞".to_owned()
            } else {
                lower.to_string()
            },
            if upper > 500 {
                "∞".to_owned()
            } else {
                upper.to_string()
            }
        )
    };

    let time_change = Callback::from(move |s: String| {
        if let Ok(new_value) = TimeMode::from_str(&s) {
            time_signals.corr_days.update(|v| *v = 2);
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
                    is_tournament=false
                    time_signals
                    on_value_change=time_change
                    allowed_values
                />
            </div>
            <div class="flex gap-1 p-1">
                {t!(i18n, home.custom_game.casual)}
                <SimpleSwitch
                    checked=params.rated
                    optional_action=add_expansions
                    disabled=untimed_no_rated
                /> {t!(i18n, home.custom_game.rated)}
            </div>
            <div class="flex gap-1 p-1">
                Base <SimpleSwitch checked=params.with_expansions optional_action=make_unrated />MLP
            </div>

            <Show when=move || opponent().is_none()>
                <div class="flex gap-1 p-1">
                    {t!(i18n, home.custom_game.private)} <SimpleSwitch checked=params.is_public />
                    {t!(i18n, home.custom_game.public)}
                </div>
                <p class="flex justify-center">{t!(i18n, home.custom_game.rating_range)}</p>
                <div class="flex justify-center w-24">{rating_string}</div>
                <div class="flex">
                    <div class="flex gap-1 mx-1">
                        <label class="flex items-center">
                            <InputSlider
                                signal_to_update=params.lower_slider
                                name="above"
                                min=-550
                                max=0
                                step=50
                            />
                        </label>
                        <label class="flex items-center">
                            <InputSlider
                                signal_to_update=params.upper_slider
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
