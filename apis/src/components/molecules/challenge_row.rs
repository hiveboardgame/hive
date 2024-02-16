use crate::common::challenge_action::ChallengeVisibility;
use crate::components::atoms::status_indicator::StatusIndicator;
use crate::components::molecules::time_row::TimeRow;
use crate::providers::api_requests::ApiRequests;
use crate::{
    components::atoms::game_type::GameType,
    components::atoms::profile_link::ProfileLink,
    functions::hostname::hostname_and_port,
    providers::{
        auth_context::AuthContext, color_scheme::ColorScheme, game_state::GameStateSignal,
    },
    responses::challenge::ChallengeResponse,
};
use hive_lib::color::ColorChoice;
use leptos::*;
use leptos_icons::*;
use leptos_router::*;
use leptos_use::use_window;

#[component]
pub fn ChallengeRow(challenge: StoredValue<ChallengeResponse>, single: bool) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let color_context = expect_context::<ColorScheme>;
    let icon = move || match challenge().color_choice {
        ColorChoice::Random => {
            view! { <Icon icon=icondata::BsHexagonHalf class="pb-[2px]"/> }
        }
        ColorChoice::White => {
            if (color_context().prefers_dark)() {
                view! { <Icon icon=icondata::BsHexagonFill class="fill-white pb-[2px]"/> }
            } else {
                view! { <Icon icon=icondata::BsHexagon class="stroke-black pb-[2px]"/> }
            }
        }
        ColorChoice::Black => {
            if (color_context().prefers_dark)() {
                view! { <Icon icon=icondata::BsHexagon class="stroke-white pb-[2px]"/> }
            } else {
                view! { <Icon icon=icondata::BsHexagonFill class="fill-black pb-[2px]"/> }
            }
        }
    };

    let challenge_address =
        move || format!("{}/challenge/{}", hostname_and_port(), challenge().nanoid);
    let button_ref = create_node_ref::<html::Button>();
    let copy = move |_| {
        let clipboard = use_window()
            .as_ref()
            .expect("window to exist")
            .navigator()
            .clipboard()
            .expect("to have clipboard permission");
        let _ = clipboard.write_text(&challenge_address());
        let class_list = button_ref
            .get_untracked()
            .expect("div_ref to be loaded by now")
            .class_list();
        class_list
            .remove_2("bg-ant-blue", "hover:bg-pillbug-teal")
            .expect("tw classes to exist");
        class_list
            .add_2("bg-grasshopper-green", "hover:bg-green-500")
            .expect("tw classes to be added");
    };

    let td_class = "xs:py-1 xs:px-1 sm:py-2 sm:px-2";
    let time_mode = challenge().time_mode;
    let uid = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user.id),
        _ => None,
    };
    let player = move || {
        if let (Some(uid), Some(opponent)) = (uid(), challenge().opponent) {
            if challenge().challenger.uid == uid {
                view! {
                    <div class="flex items-center">
                        <StatusIndicator username=opponent.username.to_owned()/>
                        <ProfileLink
                            username=opponent.username
                            extend_tw_classes="truncate max-w-[25px] xs:max-w-[75px] sm:max-w-[150px]"
                        />
                    </div>
                }
            } else {
                view! {
                    <div class="flex items-center">
                        <StatusIndicator username=challenge().challenger.username/>
                        <ProfileLink
                            username=challenge().challenger.username
                            extend_tw_classes="truncate max-w-[25px] xs:max-w-[75px] sm:max-w-[150px]"
                        />
                    </div>
                }
            }
        } else {
            view! {
                <div class="flex items-center">
                    <StatusIndicator username=challenge().challenger.username/>
                    <ProfileLink
                        username=challenge().challenger.username
                        extend_tw_classes="truncate max-w-[25px] xs:max-w-[75px] sm:max-w-[150px]"
                    />
                </div>
            }
        }
    };

    let rating = move || {
        if let (Some(uid), Some(opponent)) = (uid(), challenge().opponent) {
            if challenge().challenger.uid == uid {
                view! { <p>{opponent.rating}</p> }
            } else {
                view! { <p>{challenge().challenger.rating}</p> }
            }
        } else {
            view! { <p>{challenge().challenger.rating}</p> }
        }
    };

    view! {
        <tr class="dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light text-center items-center cursor-pointer max-w-fit">
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
                    <GameType game_type=challenge().game_type/>
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <TimeRow
                        time_mode=time_mode
                        time_base=challenge().time_base
                        increment=challenge().time_increment
                        extend_tw_classes="break-words max-w-[40px] sm:max-w-fit sm:whitespace-nowrap"
                    />
                </div>
            </td>
            <td class=td_class>
                <div class="flex justify-center items-center">
                    <span class="font-bold">{if challenge().rated { "YES" } else { "NO" }}</span>
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
                                        ref=button_ref
                                        on:click=copy
                                        class="bg-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white py-1 px-1 rounded focus:outline-none focus:shadow-outline m-1"
                                    >
                                        <Icon icon=icondata::AiCopyOutlined class="w-6 h-6"/>
                                    </button>
                                </Show>
                                <button
                                    on:click=move |_| {
                                        ApiRequests::new().challenge_cancel(challenge().nanoid)
                                    }

                                    class="bg-ladybug-red hover:bg-red-400 transform transition-transform duration-300 active:scale-95 text-white py-1 px-1 rounded focus:outline-none focus:shadow-outline m-1"
                                >
                                    <Icon icon=icondata::IoCloseSharp class="w-6 h-6"/>
                                </button>
                            }
                        }
                    >

                        <button
                            on:click=move |_| {
                                match (auth_context.user)() {
                                    Some(Ok(Some(_))) => {
                                        let mut game_state = expect_context::<GameStateSignal>();
                                        game_state.full_reset();
                                        ApiRequests::new().challenge_accept(challenge().nanoid);
                                    }
                                    _ => {
                                        let navigate = use_navigate();
                                        navigate("/login", Default::default());
                                    }
                                }
                            }

                            class="bg-ant-blue hover:bg-pillbug-teal transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 px-1 rounded focus:outline-none focus:shadow-outline m-1"
                        >
                            <Icon icon=icondata::AiCheckOutlined class="w-6 h-6"/>

                        </button>
                        {if challenge().opponent.is_some() {
                            view! {
                                <button
                                    on:click=move |_| {
                                        match (auth_context.user)() {
                                            Some(Ok(Some(_))) => {
                                                ApiRequests::new().challenge_cancel(challenge().nanoid);
                                            }
                                            _ => {
                                                let navigate = use_navigate();
                                                navigate("/login", Default::default());
                                            }
                                        }
                                    }

                                    class="bg-ladybug-red hover:bg-red-400 transform transition-transform duration-300 active:scale-95 text-white font-bold py-1 px-1 rounded focus:outline-none focus:shadow-outline m-1"
                                >
                                    <Icon icon=icondata::IoCloseSharp class="w-6 h-6"/>

                                </button>
                            }
                                .into_view()
                        } else {
                            view! {}.into_view()
                        }}

                    </Show>
                </div>
            </td>
        </tr>
    }
}
