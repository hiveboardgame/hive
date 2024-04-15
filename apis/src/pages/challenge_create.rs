use crate::{
    common::challenge_action::{ChallengeAction, ChallengeVisibility},
    components::atoms::select_options::SelectOption,
    providers::{api_requests::ApiRequests, auth_context::AuthContext, color_scheme::ColorScheme},
};
use hive_lib::{color::ColorChoice, game_type::GameType};
use leptos::*;
use leptos_icons::*;
use leptos_use::use_debounce_fn_with_arg;
use shared_types::{
    game_speed::GameSpeed,
    time_mode::{CorrespondenceMode, TimeMode},
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
    let color_context = expect_context::<ColorScheme>;
    let icon = move |color_choice: ColorChoice| {
        move || match color_choice {
            ColorChoice::Random => {
                view! { <Icon icon=icondata::BsHexagonHalf class="w-full h-full"/> }
            }
            ColorChoice::White => {
                if (color_context().prefers_dark)() {
                    view! { <Icon icon=icondata::BsHexagonFill class="fill-white w-full h-full"/> }
                } else {
                    view! { <Icon icon=icondata::BsHexagon class="stroke-black stroke-1 w-full h-full"/> }
                }
            }
            ColorChoice::Black => {
                if (color_context().prefers_dark)() {
                    view! { <Icon icon=icondata::BsHexagon class="stroke-white stroke-1 w-full h-full"/> }
                } else {
                    view! { <Icon icon=icondata::BsHexagonFill class="fill-black w-full h-full"/> }
                }
            }
        }
    };
    let time_control = RwSignal::new(TimeMode::RealTime);
    let corr_mode = RwSignal::new(CorrespondenceMode::DaysPerMove);
    let band_upper = RwSignal::new(550_i32);
    let band_lower = RwSignal::new(-550_i32);
    let corr_days = RwSignal::new(2_i32);
    let step_min = RwSignal::new(10_i32);
    let step_sec = RwSignal::new(10_i32);
    let total_seconds = Signal::derive(move || {
        let step = step_min();
        (match step {
            1..=20 => step,
            21 => step + 4,
            22 => step + 8,
            23..=32 => (step - 20) * 15,
            i32::MIN..=0_i32 | 33_i32..=i32::MAX => unreachable!(),
        }) * 60
    });
    let sec_per_move = Signal::derive(move || {
        let step = step_sec();
        match step {
            0..=20 => step,
            21 => step + 4,
            22 => step + 8,
            23..=32 => (step - 20) * 15,
            i32::MIN..=-1_i32 | 33_i32..=i32::MAX => unreachable!(),
        }
    });

    let create_challenge = move |color_choice| {
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
            .update_untracked(|v| *v = time_control.get_untracked());
        match (params.time_mode)() {
            TimeMode::Untimed => {
                params.time_base.update_value(|v| *v = None);
                params.time_increment.update_value(|v| *v = None);
            }
            TimeMode::RealTime => {
                params
                    .time_base
                    .update_value(|v| *v = Some(total_seconds.get_untracked()));
                params
                    .time_increment
                    .update_value(|v| *v = Some(sec_per_move.get_untracked()));
            }
            TimeMode::Correspondence => {
                match corr_mode.get_untracked() {
                    CorrespondenceMode::DaysPerMove => {
                        params
                            .time_increment
                            .update_value(|v| *v = Some(corr_days.get_untracked() * 86400));
                        params.time_base.update_value(|v| *v = None);
                    }
                    CorrespondenceMode::TotalTimeEach => {
                        params.time_increment.update_value(|v| *v = None);
                        params
                            .time_base
                            .update_value(|v| *v = Some(corr_days.get_untracked() * 86400));
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
        let challenge_action = ChallengeAction::Create {
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
        api.challenge(challenge_action);
        params
            .visibility
            .update(|v| *v = ChallengeVisibility::Public);
        params.game_type.update(|v| *v = GameType::MLP);
        params.rated.set(true);
        close(());
    };

    let buttons_style =
        "my-1 p-1 transform transition-transform duration-300 active:scale-95 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded";
    let disable_rated = move || {
        if (params.game_type)() == GameType::Base || time_control() == TimeMode::Untimed {
            return true;
        }
        false
    };

    let active_color = move |b| {
        if b {
            "bg-ant-blue"
        } else {
            "bg-odd-light dark:bg-gray-700"
        }
    };
    let slider_style="appearance-none accent-gray-500 dark:accent-gray-400 rounded-full bg-odd-light dark:bg-gray-700 h-4 w-64 p-1";
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
    let throttled_slider = move |signal_to_update| {
        use_debounce_fn_with_arg(update_from_slider(signal_to_update), 50.0)
    };

    view! {
        <div class="flex flex-col m-2 w-80 sm:w-96 items-center">
            <div class=move || {
                opponent().map_or("hidden", |_| "block")
            }>"Opponent: " {opponent()}</div>
            <div class="flex">
                <label class="mr-1">
                    Time Control:
                    <select
                        class="bg-odd-light dark:bg-gray-700"
                        name="Time Control"
                        on:change=move |ev| {
                            if let Ok(new_value) = TimeMode::from_str(&event_target_value(&ev)) {
                                params.visibility.update(|v| *v = ChallengeVisibility::Public);
                                params.game_type.update(|v| *v = GameType::MLP);
                                corr_days.update_untracked(|v| *v = 2);
                                if new_value == TimeMode::Untimed {
                                    params.rated.set(false);
                                } else {
                                    params.rated.set(true);
                                }
                                time_control.update(|v| *v = new_value);
                            }
                        }
                    >

                        <SelectOption value=time_control is="Real Time"/>
                        <SelectOption value=time_control is="Correspondence"/>
                        <SelectOption value=time_control is="Untimed"/>
                    </select>
                </label>
            </div>
            <Show when=move || time_control() != TimeMode::Untimed>
                <Show
                    when=move || time_control() == TimeMode::RealTime
                    fallback=move || {
                        view! {
                            <div class="flex flex-col justify-center items-center mb-1">

                                <label class="flex flex-col items-center">
                                    <div class="flex gap-1 p-1">
                                        <select
                                            class="bg-odd-light dark:bg-gray-700 mr-1"
                                            name="Correspondence Mode"
                                            on:change=move |ev| {
                                                if let Ok(new_value) = CorrespondenceMode::from_str(
                                                    &event_target_value(&ev),
                                                ) {
                                                    corr_mode.update(|v| *v = new_value);
                                                }
                                            }
                                        >

                                            <SelectOption value=corr_mode is="Days per move"/>
                                            <SelectOption value=corr_mode is="Total time each"/>

                                        </select>
                                        <div class="w-4">{corr_days}</div>
                                    </div>
                                    <input
                                        on:input=move |evt| {
                                            throttled_slider(corr_days)(evt);
                                        }

                                        type="range"
                                        name="Correspondence"
                                        min="1"
                                        max="14"
                                        prop:value=corr_days
                                        step="1"
                                        class=slider_style
                                    />
                                </label>
                            </div>
                        }
                    }
                >

                    <div class="flex flex-col justify-center">
                        <label class="flex-col items-center">
                            <div>
                                {move || format!("Minutes per side: {}", total_seconds() / 60)}
                            </div>
                            <input
                                on:input=move |evt| {
                                    throttled_slider(step_min)(evt);
                                }

                                type="range"
                                name="minutes"
                                min="1"
                                max="32"
                                prop:value=step_min
                                step="1"
                                class=slider_style
                            />
                        </label>
                        <label class="flex-col items-center">
                            <div>{move || format!("Increment in sec: {}", sec_per_move())}</div>
                            <input
                                on:input=move |evt| {
                                    throttled_slider(step_sec)(evt);
                                }

                                type="range"
                                name="increment"
                                min="0"
                                max="32"
                                prop:value=step_sec
                                step="1"
                                class=slider_style
                            />
                        </label>
                    </div>
                </Show>
            </Show>
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
                    MLP
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
                <div class="w-24 flex justify-center">{rating_string}</div>
                <div class="flex">
                    <div class="flex mx-1 gap-1">
                        <label class="flex items-center">
                            <input
                                on:input=move |evt| {
                                    throttled_slider(band_lower)(evt);
                                }

                                type="range"
                                name="above"
                                min="-550"
                                max="0"
                                prop:value=band_lower
                                step="50"
                                class="appearance-none accent-gray-500 dark:accent-gray-400 rounded-full bg-odd-light dark:bg-gray-700 h-4 p-1"
                            />
                        </label>
                        <label class="flex items-center">
                            <input
                                on:input=move |evt| {
                                    throttled_slider(band_upper)(evt);
                                }

                                type="range"
                                name="below"
                                max="550"
                                min="0"
                                prop:value=band_upper
                                step="50"
                                class="appearance-none accent-gray-500 dark:accent-gray-400 rounded-full bg-odd-light dark:bg-gray-700 h-4 p-1"
                            />
                        </label>
                    </div>
                </div>
            </div>
            <div class="flex justify-center items-baseline">
                <button
                    title="White"
                    class=format!(
                        "m-1 h-[4.5rem] w-16 bg-odd-light dark:bg-gray-700 {buttons_style}",
                    )

                    on:click=move |_| { create_challenge(ColorChoice::White) }
                >
                    {icon(ColorChoice::White)}
                </button>
                <button
                    title="Random Side"
                    class=format!("m-1 h-20 w-20 bg-odd-light dark:bg-gray-700 {buttons_style}")

                    on:click=move |_| { create_challenge(ColorChoice::Random) }
                >
                    {icon(ColorChoice::Random)}
                </button>
                <button
                    title="Black"
                    class=format!(
                        "m-1 h-[4.5rem] w-16 bg-odd-light dark:bg-gray-700 {buttons_style}",
                    )

                    on:click=move |_| { create_challenge(ColorChoice::Black) }
                >
                    {icon(ColorChoice::Black)}
                </button>
            </div>
        </div>
    }
}

fn update_from_slider(signal_to_update: RwSignal<i32>) -> impl Fn(web_sys::Event) + Clone {
    move |evt: web_sys::Event| {
        if let Ok(value) = event_target_value(&evt).parse::<i32>() {
            signal_to_update.update(|v| *v = value)
        }
    }
}
