use crate::common::TimeParamsStoreFields;
use crate::components::atoms::input_slider::InputSliderWithCallback;
use crate::components::atoms::simple_switch::SimpleSwitchWithCallback;
use crate::i18n::*;
use crate::providers::{ClientApi, AuthContext, ChallengeParams, ChallengeParamsStoreFields};
use crate::{
    common::ChallengeAction,
    components::{
        atoms::create_challenge_button::CreateChallengeButton, organisms::time_select::TimeSelect,
    },
};
use hive_lib::{ColorChoice, GameType};
use leptos::prelude::*;
use reactive_stores::Store;
use shared_types::{ChallengeDetails, ChallengeVisibility, GameSpeed, TimeMode};

#[component]
pub fn ChallengeCreate(#[prop(optional)] opponent: Option<String>) -> impl IntoView {
    let i18n = use_i18n();
    let params = expect_context::<Store<ChallengeParams>>();
    let client_api = expect_context::<ClientApi>();
    let auth_context = expect_context::<AuthContext>();
    let opponent = StoredValue::new(opponent);
    let opponent_exists = opponent.with_value(|o| o.is_some());
    let create_challenge = Callback::new(move |color_choice| {
        let (upper_rating, lower_rating) = auth_context.user.with(|acc_opt| {
            if let Some(account) = acc_opt {
                let time_data = params.time_signals().get();
                let game_speed =
                    GameSpeed::from_base_increment(time_data.base(), time_data.increment());
                let rating = account.user.rating_for_speed(&game_speed);

                let upper_slider = params.upper_slider().get();
                let upper = if upper_slider > 500 || opponent_exists {
                    None
                } else {
                    // TODO: Make rating update in realtime, currently it becomes stale
                    Some((rating as i32).saturating_add(upper_slider))
                };

                let lower_slider = params.lower_slider().get();
                let lower = if lower_slider < -500 || opponent_exists {
                    None
                } else {
                    // TODO: Make rating update in realtime, currently it becomes stale
                    Some((rating as i32).saturating_add(lower_slider))
                };

                (upper, lower)
            } else {
                (None, None)
            }
        });
        let details = ChallengeDetails {
            rated: params.rated().get_untracked(),
            game_type: if params.with_expansions().get_untracked() {
                GameType::MLP
            } else {
                GameType::Base
            },
            visibility: if !opponent_exists {
                if params.is_public().get_untracked() {
                    ChallengeVisibility::Public
                } else {
                    ChallengeVisibility::Private
                }
            } else {
                ChallengeVisibility::Direct
            },
            opponent: opponent.get_value(),
            color_choice,
            time_mode: params.time_signals().time_mode().get_untracked(),
            time_base: params.time_signals().with(|ts| ts.base()),
            time_increment: params.time_signals().with(|ts| ts.increment()),
            band_upper: upper_rating,
            band_lower: lower_rating,
        };
        let challenge_action = ChallengeAction::Create(details);
        let api = client_api;
        api.challenge(challenge_action);
    });

    let rating_string = move || {
        let lower = params.lower_slider().get();
        let upper = params.upper_slider().get();
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

    let time_change = Callback::new(move |t: TimeMode| {
        params.time_signals().corr_days().update(|v| *v = 2);
        if t == TimeMode::Untimed {
            params.rated().set(false);
        }
        params.time_signals().time_mode().update(|v| *v = t);
    });
    let allowed_values = vec![
        TimeMode::RealTime,
        TimeMode::Correspondence,
        TimeMode::Untimed,
    ];
    let with_expansions_callback = Callback::new(move |()| {
        params.with_expansions().update(|b| *b = !*b);
        if !params.with_expansions().get() {
            params.rated().set(false)
        }
    });
    let rated_callback = Callback::new(move |()| {
        params.rated().update(|b| *b = !*b);
        if params.rated().get() {
            params.with_expansions().set(true)
        }
    });
    let untimed_no_rated =
        Signal::derive(move || params.time_signals().time_mode().get() == TimeMode::Untimed);
    let upper_slider_callback = Callback::new(move |new: i32| {
        params.upper_slider().update(|v| *v = new);
    });
    let lower_slider_callback = Callback::new(move |new: i32| {
        params.lower_slider().update(|v| *v = new);
    });
    let is_public_callback = Callback::new(move |()| {
        params.is_public().update(|b| *b = !*b);
    });
    view! {
        <div class="flex flex-col items-center w-72 xs:m-2 xs:w-80 sm:w-96">
            <Show when=move || opponent_exists>
                <div class="block">"Opponent: " {opponent.get_value()}</div>
            </Show>
            <div class="flex flex-col items-center">
                <TimeSelect is_tournament=false params on_value_change=time_change allowed_values />
            </div>
            <div class="flex gap-1 p-1">
                {t!(i18n, home.custom_game.casual)}
                <SimpleSwitchWithCallback
                    checked=params.rated().into()
                    action=rated_callback
                    disabled=untimed_no_rated
                /> {t!(i18n, home.custom_game.rated)}
            </div>
            <div class="flex gap-1 p-1">
                Basic
                <SimpleSwitchWithCallback
                    checked=params.with_expansions().into()
                    action=with_expansions_callback
                />Full
            </div>

            <Show when=move || !opponent_exists>
                <div class="flex gap-1 p-1">
                    {t!(i18n, home.custom_game.private)}
                    <SimpleSwitchWithCallback
                        checked=params.is_public().into()
                        action=is_public_callback
                    /> {t!(i18n, home.custom_game.public)}
                </div>
                <p class="flex justify-center">{t!(i18n, home.custom_game.rating_range)}</p>
                <div class="flex justify-center w-24">{rating_string}</div>
                <div class="flex">
                    <div class="flex gap-1 mx-1">
                        <label class="flex items-center">
                            <InputSliderWithCallback
                                signal=params.lower_slider().into()
                                callback=lower_slider_callback
                                name="above"
                                min=-550
                                max=0
                                step=50
                            />
                        </label>
                        <label class="flex items-center">
                            <InputSliderWithCallback
                                signal=params.upper_slider().into()
                                callback=upper_slider_callback
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
                <form method="dialog">
                    <CreateChallengeButton
                        color_choice=StoredValue::new(ColorChoice::White)
                        create_challenge
                    />
                    <CreateChallengeButton
                        color_choice=StoredValue::new(ColorChoice::Random)
                        create_challenge
                    />
                    <CreateChallengeButton
                        color_choice=StoredValue::new(ColorChoice::Black)
                        create_challenge
                    />
                </form>
            </div>
        </div>
    }
}
