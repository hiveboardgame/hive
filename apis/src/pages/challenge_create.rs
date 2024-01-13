use crate::{
    common::challenge_action::{ChallengeAction, ChallengeVisibility},
    components::atoms::select_options::SelectOption,
    providers::{api_requests::ApiRequests, color_scheme::ColorScheme},
};
use hive_lib::{color::ColorChoice, game_type::GameType};
use leptos::*;
use leptos_icons::{
    BsIcon::{BsHexagon, BsHexagonFill, BsHexagonHalf},
    Icon,
};
use shared_types::time_mode::TimeMode;
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub struct ChallengeParams {
    pub rated: RwSignal<bool>,
    pub game_type: RwSignal<GameType>,
    pub visibility: RwSignal<ChallengeVisibility>,
    pub opponent: RwSignal<Option<String>>,
    pub color_choice: RwSignal<ColorChoice>,
    pub time_mode: RwSignal<TimeMode>,
    pub time_base: RwSignal<Option<i32>>,
    pub time_increment: RwSignal<Option<i32>>,
}

#[component]
pub fn ChallengeCreate(close: Callback<()>) -> impl IntoView {
    let params = ChallengeParams {
        rated: RwSignal::new(true),
        game_type: RwSignal::new(GameType::MLP),
        visibility: RwSignal::new(ChallengeVisibility::Public),
        opponent: RwSignal::new(None),
        color_choice: RwSignal::new(ColorChoice::Random),
        time_mode: RwSignal::new(TimeMode::RealTime),
        time_base: RwSignal::new(Some(10)),
        time_increment: RwSignal::new(Some(10)),
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
                view! { <Icon icon=Icon::from(BsHexagonHalf) class="w-full h-full"/> }
            }
            ColorChoice::White => {
                if (color_context().prefers_dark)() {
                    view! { <Icon icon=Icon::from(BsHexagonFill) class="fill-white w-full h-full"/> }
                } else {
                    view! { <Icon icon=Icon::from(BsHexagon) class="stroke-black stroke-1 w-full h-full"/> }
                }
            }
            ColorChoice::Black => {
                if (color_context().prefers_dark)() {
                    view! { <Icon icon=Icon::from(BsHexagon) class="stroke-white stroke-1 w-full h-full"/> }
                } else {
                    view! { <Icon icon=Icon::from(BsHexagonFill) class="fill-black w-full h-full"/> }
                }
            }
        }
    };
    let time_control = RwSignal::new(TimeMode::RealTime);
    let min_rating = RwSignal::new(-500_i16);
    let max_rating = RwSignal::new(500_i16);
    let days_per_move = RwSignal::new(2_i32);
    let total_days_per_player = RwSignal::new(0_i32);
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

    let min_rating_slider = {
        move |evt| {
            let value = event_target_value(&evt).parse::<i16>().unwrap();
            min_rating.update(|v| *v = value);
        }
    };
    let max_rating_slider = {
        move |evt| {
            let value = event_target_value(&evt).parse::<i16>().unwrap();
            max_rating.update(|v| *v = value);
        }
    };
    let days_slider = {
        move |evt| {
            let value = event_target_value(&evt).parse::<i32>().unwrap();
            days_per_move.update(|v| *v = value);
            if value != 0 {
                total_days_per_player.update(|v| *v = 0);
            }
        }
    };
    let total_slider = {
        move |evt| {
            let value = event_target_value(&evt).parse::<i32>().unwrap();
            total_days_per_player.update(|v| *v = value);
            if value != 0 {
                days_per_move.update(|v| *v = 0);
            }
        }
    };
    let incr_slider = {
        move |evt| {
            let value = event_target_value(&evt).parse::<i32>().unwrap();
            step_sec.update(|v| *v = value);
        }
    };
    let minute_slider = {
        move |evt| {
            let value = event_target_value(&evt).parse::<i32>().unwrap();
            step_min.update(|v| *v = value);
        }
    };

    let create_challenge = move |color_choice| {
        params.color_choice.update(|p| *p = color_choice);
        let api = ApiRequests::new();
        params
            .time_mode
            .update_untracked(|v| *v = time_control.get_untracked());
        match (params.time_mode)() {
            TimeMode::Untimed => {
                params.time_base.update_untracked(|v| *v = None);
                params.time_increment.update_untracked(|v| *v = None);
            }
            TimeMode::RealTime => {
                params
                    .time_base
                    .update_untracked(|v| *v = Some(total_seconds.get_untracked()));
                params
                    .time_increment
                    .update_untracked(|v| *v = Some(sec_per_move.get_untracked()));
            }
            TimeMode::Correspondence => {
                if days_per_move.get_untracked() != 0 {
                    params
                        .time_increment
                        .update_untracked(|v| *v = Some(days_per_move.get_untracked() * 86400));
                    params.time_base.update_untracked(|v| *v = None);
                } else {
                    params.time_increment.update_untracked(|v| *v = None);
                    params.time_base.update_untracked(|v| {
                        *v = Some(total_days_per_player.get_untracked() * 86400)
                    });
                };
            }
        };
        let challenge_action = ChallengeAction::Create {
            rated: params.rated.get_untracked(),
            game_type: params.game_type.get_untracked(),
            visibility: params.visibility.get_untracked(),
            opponent: params.opponent.get_untracked(),
            color_choice: params.color_choice.get_untracked(),
            time_mode: params.time_mode.get_untracked().to_string(),
            time_base: params.time_base.get_untracked(),
            time_increment: params.time_increment.get_untracked(),
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
        "my-1 p-1 duration-300 hover:shadow-xl dark:hover:shadow dark:hover:shadow-gray-500 drop-shadow-lg dark:shadow-gray-600 rounded";
    let disable_rated = move || {
        if (params.game_type)() == GameType::Base || time_control() == TimeMode::Untimed {
            return true;
        }
        false
    };
    let disable_game_create = move || {
        if days_per_move() == 0
            && total_days_per_player() == 0
            && time_control() == TimeMode::Correspondence
        {
            return true;
        }
        false
    };

    let active_color = move |b| {
        if b {
            "bg-blue-500"
        } else {
            "bg-odd-light dark:bg-gray-700"
        }
    };
    let slider_style="appearance-none accent-gray-500 dark:accent-gray-400 rounded-full bg-odd-light dark:bg-gray-700";

    view! {
        <div class="flex flex-col m-2 flex-shrink-0 flex-grow-0">
            <div class="flex">
                <label for="time-control" class="mr-1">
                    Time Control:
                </label>
                <select
                    class="bg-odd-light dark:bg-gray-700"
                    name="Time Control"
                    id="time-control"
                    on:change=move |ev| {
                        let new_value = TimeMode::from_str(&event_target_value(&ev))
                            .expect("Valid TimeMode");
                        params.visibility.update(|v| *v = ChallengeVisibility::Public);
                        params.game_type.update(|v| *v = GameType::MLP);
                        days_per_move.update_untracked(|v| *v = 2);
                        if new_value == TimeMode::Untimed {
                            params.rated.set(false);
                        } else {
                            params.rated.set(true);
                        }
                        time_control.update(|v| *v = new_value);
                    }
                >

                    <SelectOption value=time_control is="Real Time"/>
                    <SelectOption value=time_control is="Correspondence"/>
                    <SelectOption value=time_control is="Untimed"/>
                </select>
            </div>
            <Show when=move || time_control() != TimeMode::Untimed>
                <Show
                    when=move || time_control() == TimeMode::RealTime
                    fallback=move || {
                        view! {
                            <div class="flex flex-col justify-center items-center mb-1">
                                <label for="corr">
                                    {move || format!("Days per move: {}", days_per_move())}
                                </label>
                                <input
                                    on:input=days_slider
                                    type="range"
                                    id="corr"
                                    name="Correspondence"
                                    min="0"
                                    max="14"
                                    prop:value=days_per_move
                                    step="1"
                                    class=slider_style
                                />
                                <label for="corr">
                                    {move || {
                                        format!(
                                            "total_days_per_player: {}",
                                            total_days_per_player(),
                                        )
                                    }}

                                </label>
                                <input
                                    on:input=total_slider
                                    type="range"
                                    id="corr"
                                    name="Correspondence"
                                    min="0"
                                    max="14"
                                    prop:value=total_days_per_player
                                    step="1"
                                    class=slider_style
                                />
                            </div>
                        }
                    }
                >

                    <div class="flex flex-col justify-center">
                        <label for="minutes">
                            {move || format!("Minutes per side: {}", total_seconds() / 60)}
                        </label>
                        <input
                            on:input=minute_slider
                            type="range"
                            id="minutes"
                            name="minutes"
                            min="1"
                            max="32"
                            prop:value=step_min
                            step="1"
                            class=slider_style
                        />
                        <label for="increment">
                            {move || format!("Increment in sec: {}", sec_per_move())}
                        </label>
                        <input
                            on:input=incr_slider
                            type="range"
                            id="increment"
                            name="increment"
                            min="0"
                            max="32"
                            prop:value=step_sec
                            step="1"
                            class=slider_style
                        />
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
            <div class="flex justify-center">
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
            <div>
                <p class="flex justify-center">Rating range</p>
                <p class="flex">
                    <input
                        on:input=min_rating_slider
                        type="range"
                        id="below"
                        name="below"
                        min="-500"
                        max="0"
                        prop:value=min_rating
                        step="50"
                        class=slider_style
                    />
                    <p class="flex mx-1 w-[5.5rem] justify-center">
                        <label for="below" class="whitespace-nowrap">
                            {move || format!("{} /", min_rating())}

                        </label>
                        <label for="above" class="whitespace-nowrap">
                            {move || format!(" +{}", max_rating())}
                        </label>
                    </p>
                    <input
                        on:input=max_rating_slider
                        type="range"
                        id="above"
                        name="above"
                        min="0"
                        max="500"
                        prop:value=max_rating
                        step="50"
                        class=slider_style
                    />
                </p>
            </div>
            <div class="flex justify-center items-baseline">
                <button
                    prop:disabled=disable_game_create
                    class=format!(
                        "disabled:opacity-25 disabled:cursor-not-allowed m-1 h-[4.5rem] w-16 bg-odd-light dark:bg-gray-700 {buttons_style}",
                    )

                    on:click=move |_| { create_challenge(ColorChoice::White) }
                >
                    {icon(ColorChoice::White)}
                </button>
                <button
                    prop:disabled=disable_game_create
                    class=format!(
                        "disabled:opacity-25 disabled:cursor-not-allowed m-1 h-20 w-20 bg-odd-light dark:bg-gray-700 {buttons_style}",
                    )

                    on:click=move |_| { create_challenge(ColorChoice::Random) }
                >
                    {icon(ColorChoice::Random)}
                </button>
                <button
                    prop:disabled=disable_game_create
                    class=format!(
                        "disabled:opacity-25 disabled:cursor-not-allowed m-1 h-[4.5rem] w-16 bg-odd-light dark:bg-gray-700 {buttons_style}",
                    )

                    on:click=move |_| { create_challenge(ColorChoice::Black) }
                >
                    {icon(ColorChoice::Black)}
                </button>
            </div>
        </div>
    }
}
