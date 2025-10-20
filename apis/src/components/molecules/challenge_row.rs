use crate::components::atoms::status_indicator::StatusIndicator;
use crate::components::molecules::time_row::TimeRow;
use crate::i18n::*;
use crate::providers::Config;
use crate::websocket::new_style::client::ClientApi;
use crate::{
    components::atoms::game_type::GameType, components::atoms::profile_link::ProfileLink,
    functions::hostname::hostname_and_port, responses::ChallengeResponse,
};
use hive_lib::ColorChoice;
use leptos::either::Either;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_use::{use_interval_fn_with_options, use_window, UseIntervalFnOptions};
use shared_types::{ChallengeVisibility, TimeInfo};
use uuid::Uuid;

const BUTTON_BASE_CLASSES: &str = "px-1 py-1 m-1 text-white rounded transition-transform duration-300 transform active:scale-95 focus:outline-none focus:shadow-outline font-bold";

#[component]
pub fn ChallengeRow(
    challenge: ChallengeResponse,
    single: bool,
    uid: Option<Uuid>,
) -> impl IntoView {
    let i18n = use_i18n();
    let config = expect_context::<Config>().0;
    let client_api = expect_context::<ClientApi>();
    let challenge_id = StoredValue::new(challenge.challenge_id);
    let visibility = StoredValue::new(challenge.visibility);
    let color_choice = StoredValue::new(challenge.color_choice);
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
    let time_mode = challenge.time_mode;
    let (username, patreon, bot, rating) =
        if let (Some(uid), Some(ref opponent)) = (uid, &challenge.opponent) {
            if challenge.challenger.uid == uid {
                (
                    opponent.username.as_str(),
                    opponent.patreon,
                    opponent.bot,
                    opponent.rating_for_speed(&challenge.speed),
                )
            } else {
                (
                    challenge.challenger.username.as_str(),
                    challenge.challenger.patreon,
                    challenge.challenger.bot,
                    challenge.challenger_rating,
                )
            }
        } else {
            (
                challenge.challenger.username.as_str(),
                challenge.challenger.patreon,
                challenge.challenger.bot,
                challenge.challenger_rating,
            )
        };

    let time_info = TimeInfo {
        mode: time_mode,
        base: challenge.time_base,
        increment: challenge.time_increment,
    };
    view! {
        <tr class="items-center text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light max-w-fit">
            <td class=td_class>
                <div>
                    <Icon icon=Signal::derive(icon) attr:class=Signal::derive(icon_class) />
                </div>
            </td>
            <td class=format!("w-10 sm:w-24 {td_class}")>
                <div class="flex justify-center items-center">
                    <div class="flex items-center">
                        <StatusIndicator username=username.to_string() />
                        <ProfileLink
                            username=username.to_string()
                            patreon
                            bot
                            extend_tw_classes="truncate max-w-[60px] xs:max-w-[80px] sm:max-w-[120px] md:max-w-[140px] lg:max-w-[160px]"
                        />
                    </div>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <p>{rating}</p>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <GameType game_type=challenge.game_type />
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
                            if challenge.rated {
                                t_string!(i18n, home.challenge_details.rated.yes)
                            } else {
                                t_string!(i18n, home.challenge_details.rated.no)
                            }
                        }}

                    </span>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <Show
                        when=move || { uid != Some(challenge.challenger.uid) }

                        fallback=move || {
                            view! {
                                <Show when=move || {
                                    visibility.with_value(|v| *v == ChallengeVisibility::Private)
                                        && !single
                                }>
                                    <button on:click=copy class=copy_button_class>
                                        <Icon
                                            icon=icondata_ai::AiCopyOutlined
                                            attr:class="size-6"
                                        />
                                    </button>
                                </Show>
                                <button
                                    on:click=move |_| {
                                        let api = client_api;
                                        let id = challenge_id.get_value();
                                        api.challenge_cancel(id);
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
                                let api = client_api;
                                let id = challenge_id.get_value();
                                api.challenge_accept(id);
                            }
                            class=accept_button_classes.get_value()
                        >
                            <Icon icon=icondata_ai::AiCheckOutlined attr:class="size-6" />

                        </button>
                        {if challenge.opponent.is_some() {
                            Either::Left(
                                view! {
                                    <button
                                        on:click=move |_| {
                                            let api = client_api;
                                            let id = challenge_id.get_value();
                                            api.challenge_cancel(id);
                                        }
                                        class=cancel_button_classes.get_value()
                                    >
                                        <Icon icon=icondata_io::IoCloseSharp attr:class="size-6" />

                                    </button>
                                },
                            )
                        } else {
                            Either::Right(view! { "" })
                        }}

                    </Show>
                </div>
            </td>
        </tr>
    }
}
