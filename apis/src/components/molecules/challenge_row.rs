use crate::components::atoms::status_indicator::StatusIndicator;
use crate::components::molecules::time_row::TimeRow;
use leptos_i18n::*;
use crate::providers::{ApiRequests, Config};
use crate::{
    components::atoms::game_type::GameType, components::atoms::profile_link::ProfileLink,
    functions::hostname::hostname_and_port, providers::AuthContext, responses::ChallengeResponse,
};
use hive_lib::ColorChoice;
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_router::hooks::use_navigate;
use leptos_use::use_window;
use shared_types::{ChallengeVisibility, TimeInfo};

#[component]
pub fn ChallengeRow(challenge: StoredValue<ChallengeResponse>, single: bool) -> impl IntoView {
    let challenge = Signal::derive(move || challenge.get_value().clone());
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let config = expect_context::<Config>().0;
    let icon = move || match challenge().color_choice {
        ColorChoice::Random => {
            view! { <Icon icon=icondata::BsHexagonHalf attr:class="pb-[2px]" /> }.into_any()
        }
        ColorChoice::White => {
            if config().prefers_dark {
                view! { <Icon icon=icondata::BsHexagonFill attr:class="fill-white pb-[2px]" /> }
                    .into_any()
            } else {
                view! { <Icon icon=icondata::BsHexagon attr:class="stroke-black pb-[2px]" /> }
                    .into_any()
            }
        }
        ColorChoice::Black => {
            if config().prefers_dark {
                view! { <Icon icon=icondata::BsHexagon attr:class="stroke-white pb-[2px]" /> }
                    .into_any()
            } else {
                view! { <Icon icon=icondata::BsHexagonFill attr:class="fill-black pb-[2px]" /> }
                    .into_any()
            }
        }
    };

    let challenge_address = move || {
        format!(
            "{}/challenge/{}",
            hostname_and_port(),
            challenge().challenge_id
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
            .remove_3(
                "dark:bg-button-twilight",
                "bg-button-dawn",
                "hover:bg-pillbug-teal",
            )
            .expect("tw classes to exist");
        class_list
            .add_2("bg-grasshopper-green", "hover:bg-green-500")
            .expect("tw classes to be added");
    };

    let td_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let time_mode = challenge().time_mode;
    let uid = move || match (auth_context.user).get() {
        Some(Ok(Some(user))) => Some(user.id),
        _ => None,
    };
    let player = move || {
        if let (Some(uid), Some(opponent)) = (uid(), challenge().opponent) {
            if challenge().challenger.uid == uid {
                view! {
                    <div class="flex items-center">
                        <StatusIndicator username=opponent.username.to_owned() />
                        <ProfileLink
                            username=opponent.username
                            patreon=opponent.patreon
                            extend_tw_classes="truncate max-w-[25px] xs:max-w-[75px] sm:max-w-[150px]"
                        />
                    </div>
                }
            } else {
                view! {
                    <div class="flex items-center">
                        <StatusIndicator username=challenge().challenger.username />
                        <ProfileLink
                            username=challenge().challenger.username
                            patreon=challenge().challenger.patreon
                            extend_tw_classes="truncate max-w-[25px] xs:max-w-[75px] sm:max-w-[150px]"
                        />
                    </div>
                }
            }
        } else {
            view! {
                <div class="flex items-center">
                    <StatusIndicator username=challenge().challenger.username />
                    <ProfileLink
                        username=challenge().challenger.username
                        patreon=challenge().challenger.patreon
                        extend_tw_classes="truncate max-w-[25px] xs:max-w-[75px] sm:max-w-[150px]"
                    />
                </div>
            }
        }
    };

    let rating = move || {
        if let (Some(uid), Some(opponent)) = (uid(), challenge().opponent) {
            if challenge().challenger.uid == uid {
                view! { <p>{opponent.rating_for_speed(&challenge().speed)}</p> }
            } else {
                view! { <p>{challenge().challenger_rating}</p> }
            }
        } else {
            view! { <p>{challenge().challenger_rating}</p> }
        }
    };

    let time_info = TimeInfo {
        mode: time_mode,
        base: challenge().time_base,
        increment: challenge().time_increment,
    };

    view! {
        <tr class="items-center text-center cursor-pointer dark:odd:bg-header-twilight dark:even:bg-reserve-twilight odd:bg-odd-light even:bg-even-light max-w-fit">
            <td class=td_class>
                <div>{icon}</div>
            </td>
            <td class=format!("w-10 sm:w-36 {td_class}")>
                <div class="flex justify-center items-center">{player}</div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">{rating}</div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <GameType game_type=challenge().game_type />
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <TimeRow
                        time_info=time_info.into()
                        extend_tw_classes="break-words max-w-[40px] sm:max-w-fit sm:whitespace-nowrap"
                    />
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <span class="font-bold">
                        {if challenge().rated {
                            t!(i18n, home.challenge_details.rated.yes).into_any()
                        } else {
                            t!(i18n, home.challenge_details.rated.no).into_any()
                        }}

                    </span>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <Show
                        when=move || {
                            let uid = uid();
                            uid != Some(challenge().challenger.uid)
                        }

                        fallback=move || {
                            view! {
                                <Show when=move || {
                                    challenge().visibility == ChallengeVisibility::Private
                                        && !single
                                }>
                                    <button
                                        node_ref=button_ref
                                        on:click=copy
                                        class="px-1 py-1 m-1 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 focus:outline-none focus:shadow-outline"
                                    >
                                        <Icon icon=icondata::AiCopyOutlined attr:class="w-6 h-6" />
                                    </button>
                                </Show>
                                <button
                                    on:click=move |_| {
                                        ApiRequests::new()
                                            .challenge_cancel(challenge().challenge_id)
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
                                match (auth_context.user).get() {
                                    Some(Ok(_)) => {
                                        ApiRequests::new()
                                            .challenge_accept(challenge().challenge_id);
                                    }
                                    _ => {
                                        let navigate = use_navigate();
                                        navigate("/login", Default::default());
                                    }
                                }
                            }

                            class="px-1 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95 focus:outline-none focus:shadow-outline"
                        >
                            <Icon icon=icondata::AiCheckOutlined attr:class="w-6 h-6" />

                        </button>
                        {if challenge().opponent.is_some() {
                            view! {
                                <button
                                    on:click=move |_| {
                                        match auth_context.user.get() {
                                            Some(Ok(_)) => {
                                                ApiRequests::new()
                                                    .challenge_cancel(challenge().challenge_id);
                                            }
                                            _ => {
                                                let navigate = use_navigate();
                                                navigate("/login", Default::default());
                                            }
                                        }
                                    }

                                    class="px-1 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform bg-ladybug-red hover:bg-red-400 active:scale-95 focus:outline-none focus:shadow-outline"
                                >
                                    <Icon icon=icondata::IoCloseSharp attr:class="w-6 h-6" />

                                </button>
                            }
                                .into_any()
                        } else {
                            view! { "" }.into_any()
                        }}

                    </Show>
                </div>
            </td>
        </tr>
    }
}
