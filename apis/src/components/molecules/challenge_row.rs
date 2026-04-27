use crate::{
    components::{
        atoms::{
            game_type::GameType,
            profile_link::ProfileLink,
            status_indicator::StatusIndicator,
        },
        molecules::time_row::TimeRow,
    },
    functions::hostname::hostname_and_port,
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext, Config},
    responses::ChallengeResponse,
};
use hive_lib::ColorChoice;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_use::{use_interval_fn_with_options, use_window, UseIntervalFnOptions};
use shared_types::{ChallengeId, ChallengeVisibility, TimeInfo};

const BUTTON_BASE_CLASSES: &str = "px-1 py-1 m-1 text-white rounded transition-transform duration-300 transform active:scale-95 focus:outline-none focus:shadow-outline font-bold";

#[component]
pub fn ChallengeRow(
    challenge: ChallengeResponse,
    single: bool,
    #[prop(default = 1)] count: usize,
    #[prop(default = Vec::new())] challenge_ids: Vec<ChallengeId>,
) -> impl IntoView {
    let ChallengeResponse {
        challenge_id,
        challenger,
        opponent,
        game_type,
        rated,
        visibility,
        color_choice,
        challenger_rating,
        time_mode,
        time_base,
        time_increment,
        speed,
        ..
    } = challenge;
    let i18n = use_i18n();
    let config = expect_context::<Config>().0;
    let api = expect_context::<ApiRequestsProvider>().0;
    let user = expect_context::<AuthContext>().user;
    let challenger_id = challenger.uid;
    let has_opponent = opponent.is_some();
    let challenge_id = StoredValue::new(challenge_id);
    let visibility = StoredValue::new(visibility);
    let all_challenge_ids = StoredValue::new(challenge_ids);
    let group_count = count;
    let color_choice = StoredValue::new(color_choice);
    let icon = move || {
        let prefers_dark = config.with(|c| c.prefers_dark);
        match color_choice.get_value() {
            ColorChoice::Random => icondata_bs::BsHexagonHalf,
            ColorChoice::White => {
                if prefers_dark {
                    icondata_bs::BsHexagonFill
                } else {
                    icondata_bs::BsHexagon
                }
            }
            ColorChoice::Black => {
                if prefers_dark {
                    icondata_bs::BsHexagon
                } else {
                    icondata_bs::BsHexagonFill
                }
            }
        }
    };
    let icon_class = move || {
        let prefers_dark = config.with(|c| c.prefers_dark);
        match color_choice.get_value() {
            ColorChoice::Random => "pb-[2px]",
            ColorChoice::White => {
                if prefers_dark {
                    "fill-white pb-[2px]"
                } else {
                    "stroke-black pb-[2px]"
                }
            }
            ColorChoice::Black => {
                if prefers_dark {
                    "stroke-white pb-[2px]"
                } else {
                    "fill-black pb-[2px]"
                }
            }
        }
    };
    let challenge_address = move || {
        format!(
            "{}/challenge/{}",
            hostname_and_port(),
            challenge_id.get_value()
        )
    };
    let copy_state = RwSignal::new(false);

    let interval = StoredValue::new(use_interval_fn_with_options(
        move || copy_state.set(false),
        2000, // 2 seconds
        UseIntervalFnOptions::default().immediate(false),
    ));

    let copy = move |_| {
        let interval = interval.get_value();
        let clipboard = use_window()
            .as_ref()
            .expect("window to exist")
            .navigator()
            .clipboard();
        let _ = clipboard.write_text(&challenge_address());
        copy_state.set(true);
        (interval.pause)();
        (interval.resume)();
    };
    let copy_button_class = move || {
        if copy_state.get() {
            format!("{BUTTON_BASE_CLASSES} bg-grasshopper-green hover:bg-green-500")
        } else {
            format!("{BUTTON_BASE_CLASSES} bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal")
        }
    };

    let td_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let accept_button_classes = StoredValue::new(format!("{BUTTON_BASE_CLASSES} bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal"));
    let cancel_button_classes = StoredValue::new(format!(
        "{BUTTON_BASE_CLASSES} bg-ladybug-red hover:bg-red-400"
    ));
    let displayed_user = Memo::new(move |_| {
        if user.with(|a| a.as_ref().map(|user| user.id)) == Some(challenger_id) {
            if let Some(opponent) = opponent.as_ref() {
                return (
                    opponent.username.clone(),
                    opponent.patreon,
                    opponent.bot,
                    opponent.rating_for_speed(&speed),
                );
            }
        }

        (
            challenger.username.clone(),
            challenger.patreon,
            challenger.bot,
            challenger_rating,
        )
    });
    let time_info = TimeInfo {
        mode: time_mode,
        base: time_base,
        increment: time_increment,
    };
    let actions = move || {
        view! {
            <Show
                when=move || user.with(|a| a.as_ref().map(|user| user.id)) != Some(challenger_id)
                fallback=move || {
                    view! {
                        {(visibility.with_value(|v| *v == ChallengeVisibility::Private) && !single)
                            .then(|| {
                                view! {
                                    <button on:click=copy class=copy_button_class>
                                        <Icon
                                            icon=icondata_ai::AiCopyOutlined
                                            attr:class="size-6"
                                        />
                                    </button>
                                }
                            })}
                        <button
                            on:click=move |_| {
                                let ids = all_challenge_ids.get_value();
                                let ids_to_cancel =
                                    if ids.is_empty() { vec![challenge_id.get_value()] } else { ids };
                                api.get().challenges_cancel(ids_to_cancel);
                            }
                            class=cancel_button_classes.get_value()
                        >
                            <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                        </button>
                    }
                }
            >
                <button
                    on:click=move |_| {
                        api.get().challenge_accept(challenge_id.get_value());
                    }
                    class=accept_button_classes.get_value()
                >
                    <Icon icon=icondata_ai::AiCheckOutlined attr:class="size-6" />
                </button>
                {has_opponent.then(|| {
                    view! {
                        <button
                            on:click=move |_| {
                                let ids = all_challenge_ids.get_value();
                                let ids_to_cancel =
                                    if ids.is_empty() { vec![challenge_id.get_value()] } else { ids };
                                api.get().challenges_cancel(ids_to_cancel);
                            }
                            class=cancel_button_classes.get_value()
                        >
                            <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />
                        </button>
                    }
                })}
            </Show>
        }
    };
    view! {
        <tr class="items-center text-center cursor-pointer max-w-fit dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light">
            <td class=td_class>
                <div class="flex items-center justify-center gap-1">
                    {(group_count > 1).then(|| {
                        view! {
                            <span class="px-1.5 py-0.5 text-[10px] xs:text-xs font-bold bg-pillbug-teal text-white rounded-full leading-tight shrink-0">
                                {format!("x{}", group_count)}
                            </span>
                        }
                    })}
                    <Icon icon=Signal::derive(icon) attr:class=Signal::derive(icon_class) />
                    <div class="flex flex-col items-center justify-center sm:hidden">
                        {actions()}
                    </div>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    {move || {
                        displayed_user
                            .with(|(username, patreon, bot, _)| {
                                view! {
                                    <div class="flex items-center justify-start gap-x-1 whitespace-nowrap">
                                        <StatusIndicator username=username.clone() />
                                        <div>
                                            <ProfileLink
                                                username=username.clone()
                                                patreon=*patreon
                                                bot=*bot
                                                extend_tw_classes="whitespace-nowrap leading-tight"
                                            />
                                        </div>
                                    </div>
                                }
                            })
                    }}
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <p>{move || displayed_user.with(|(_, _, _, rating)| *rating)}</p>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <GameType game_type />
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <TimeRow
                        time_info
                        extend_tw_classes="break-words text-xs sm:text-sm max-w-[40px] xs:max-w-[50px] sm:max-w-[60px] md:max-w-[80px] lg:max-w-[100px] whitespace-normal"
                    />
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <span class="font-bold">
                        {move || {
                            if rated {
                                t_string!(i18n, home.challenge_details.rated.yes)
                            } else {
                                t_string!(i18n, home.challenge_details.rated.no)
                            }
                        }}

                    </span>
                </div>
            </td>
            <td class="xs:py-1 xs:px-1 sm:py-2 sm:px-2 hidden sm:table-cell">
                <div class="flex justify-center items-center">{actions()}</div>
            </td>
        </tr>
    }
}
