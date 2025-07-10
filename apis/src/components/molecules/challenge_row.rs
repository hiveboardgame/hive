use crate::components::atoms::status_indicator::StatusIndicator;
use crate::components::molecules::time_row::TimeRow;
use crate::i18n::*;
use crate::providers::{ApiRequestsProvider, Config};
use crate::{
    components::atoms::game_type::GameType, components::atoms::profile_link::ProfileLink,
    functions::hostname::hostname_and_port, responses::ChallengeResponse,
};
use hive_lib::ColorChoice;
use leptos::either::Either;
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_use::use_window;
use shared_types::{ChallengeVisibility, TimeInfo};
use uuid::Uuid;

#[component]
pub fn ChallengeRow(
    challenge: ChallengeResponse,
    single: bool,
    uid: Option<Uuid>,
) -> impl IntoView {
    let i18n = use_i18n();
    let config = expect_context::<Config>().0;
    let api = expect_context::<ApiRequestsProvider>().0;
    let challenge_id = StoredValue::new(challenge.challenge_id);
    let visibility = StoredValue::new(challenge.visibility);
    let icon_data = move || match challenge.color_choice {
        ColorChoice::Random => (icondata::BsHexagonHalf, "pb-[2px]"),
        ColorChoice::White => {
            if config().prefers_dark {
                (icondata::BsHexagonFill, "fill-white pb-[2px]")
            } else {
                (icondata::BsHexagon, "stroke-black pb-[2px]")
            }
        }
        ColorChoice::Black => {
            if config().prefers_dark {
                (icondata::BsHexagon, "stroke-white pb-[2px]")
            } else {
                (icondata::BsHexagonFill, "fill-black pb-[2px]")
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
    let button_ref = NodeRef::<html::Button>::new();
    let copy = move |_| {
        let clipboard = use_window()
            .as_ref()
            .expect("window to exist")
            .navigator()
            .clipboard();
        let _ = clipboard.write_text(&challenge_address());
        let class_list = button_ref
            .get_untracked()
            .expect("div_ref to be loaded by now")
            .class_list();
        class_list
            .remove_4(
                "dark:bg-button-twilight",
                "bg-button-dawn",
                "hover:bg-pillbug-teal",
                "dark:hover:bg-pillbug-teal",
            )
            .expect("tw classes to exist");
        class_list
            .add_2("bg-grasshopper-green", "hover:bg-green-500")
            .expect("tw classes to be added");
    };

    let td_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let time_mode = challenge.time_mode;

    let challenger_username = StoredValue::new(challenge.challenger.username);
    let (username, patreon, bot, rating) =
        if let (Some(uid), Some(opponent)) = (uid, challenge.opponent.clone()) {
            if challenge.challenger.uid == uid {
                let opp = opponent.username.clone();
                (
                    opp.clone(),
                    opponent.patreon,
                    opponent.bot,
                    opponent.rating_for_speed(&challenge.speed),
                )
            } else {
                (
                    challenger_username.get_value(),
                    challenge.challenger.patreon,
                    challenge.challenger.bot,
                    challenge.challenger_rating,
                )
            }
        } else {
            (
                challenger_username.get_value(),
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
                    <Icon icon=icon_data().0 attr:class=icon_data().1 />
                </div>
            </td>
            <td class=format!("w-10 sm:w-36 {td_class}")>
                <div class="flex justify-center items-center">
                    <div class="flex items-center">
                        <StatusIndicator username=username.clone() />
                        <ProfileLink
                            username
                            patreon
                            bot
                            extend_tw_classes="truncate max-w-[25px] xs:max-w-[75px] sm-max-w-[150px]"
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
                        extend_tw_classes="break-words max-w-[40px] sm:max-w-fit sm:whitespace-nowrap"
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
                                    visibility.get_value() == ChallengeVisibility::Private
                                        && !single
                                }>
                                    <button
                                        node_ref=button_ref
                                        on:click=copy
                                        class="px-1 py-1 m-1 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 focus:outline-none focus:shadow-outline"
                                    >
                                        <Icon icon=icondata::AiCopyOutlined attr:class="w-6 h-6" />
                                    </button>
                                </Show>
                                <button
                                    on:click=move |_| {
                                        api.get().challenge_cancel(challenge_id.get_value())
                                    }

                                    class="px-1 py-1 m-1 text-white rounded transition-transform duration-300 transform bg-ladybug-red hover:bg-red-400 active:scale-95 focus:outline-none focus:shadow-outline"
                                >
                                    <Icon icon=icondata::IoCloseSharp attr:class="w-6 h-6" />
                                </button>
                            }
                        }
                    >

                        <button
                            on:click=move |_| {
                                api.get().challenge_accept(challenge_id.get_value());
                            }

                            class="px-1 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 focus:outline-none focus:shadow-outline"
                        >
                            <Icon icon=icondata::AiCheckOutlined attr:class="w-6 h-6" />

                        </button>
                        {if challenge.opponent.is_some() {
                            Either::Left(
                                view! {
                                    <button
                                        on:click=move |_| {
                                            api.get().challenge_cancel(challenge_id.get_value());
                                        }

                                        class="px-1 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform bg-ladybug-red hover:bg-red-400 active:scale-95 focus:outline-none focus:shadow-outline"
                                    >
                                        <Icon icon=icondata::IoCloseSharp attr:class="w-6 h-6" />

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
