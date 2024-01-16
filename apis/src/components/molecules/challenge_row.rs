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
use leptos::logging::log;
use leptos::*;
use leptos_icons::{
    AiIcon::AiCopyOutlined,
    BsIcon::{BsHexagon, BsHexagonFill, BsHexagonHalf},
    Icon,
};
use leptos_router::*;
use leptos_use::use_window;
use shared_types::time_mode::TimeMode;
use std::str::FromStr;

#[component]
pub fn ChallengeRow(challenge: StoredValue<ChallengeResponse>, single: bool) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let color_context = expect_context::<ColorScheme>;
    let icon = move || match challenge().color_choice {
        ColorChoice::Random => {
            view! { <Icon icon=Icon::from(BsHexagonHalf)/> }
        }
        ColorChoice::White => {
            if (color_context().prefers_dark)() {
                view! { <Icon icon=Icon::from(BsHexagonFill) class="fill-white"/> }
            } else {
                view! { <Icon icon=Icon::from(BsHexagon) class="stroke-black"/> }
            }
        }
        ColorChoice::Black => {
            if (color_context().prefers_dark)() {
                view! { <Icon icon=Icon::from(BsHexagon) class="stroke-white"/> }
            } else {
                view! { <Icon icon=Icon::from(BsHexagonFill) class="fill-black"/> }
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
            .remove_2("bg-blue-500", "hover:bg-blue-400")
            .expect("tw classes to exist");
        class_list
            .add_2("bg-green-500", "hover:bg-green-400")
            .expect("tw classes to be added");
    };

    let td_class = "py-1 px-1 md:py-2 md:px-2 lg:px-3";
    let time_mode = TimeMode::from_str(&challenge().time_mode).expect("Valid TimeMode");

    let uid = move || match (auth_context.user)() {
        Some(Ok(Some(user))) => Some(user.id),
        _ => None,
    };

    let player = move || {
        if let (Some(uid), Some(opponent)) = (uid(), challenge().opponent) {
            if challenge().challenger.uid == uid {
                view! {
                    <StatusIndicator username=opponent.username.to_owned()/>
                    <ProfileLink username=opponent.username/>
                }
            } else {
                view! {
                    <StatusIndicator username=challenge().challenger.username/>
                    <ProfileLink username=challenge().challenger.username/>
                }
            }
        } else {
            view! {
                <StatusIndicator username=challenge().challenger.username/>
                <ProfileLink username=challenge().challenger.username/>
            }
        }
    };

    view! {
        <tr class="dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light text-center items-center">
            <td class=td_class>{icon}</td>
            <td class=td_class>
                <p class="flex items-center">{player}</p>
            </td>
            <td class=td_class>{challenge().challenger.rating}</td>
            <td class=td_class>
                <GameType game_type=challenge().game_type/>
            </td>
            <td class=td_class>
                <TimeRow
                    time_mode=time_mode
                    time_base=challenge().time_base
                    increment=challenge().time_increment
                />
            </td>
            <td class=td_class>
                <span class="font-bold">{if challenge().rated { "RATED" } else { "CASUAL" }}</span>
            </td>
            <td class=td_class>
                <Show
                    when=move || {
                        let uid = uid();
                        uid != Some(challenge().challenger.uid)
                    }

                    fallback=move || {
                        view! {
                            <div class="justify-center flex">
                                <button
                                    on:click=move |_| {
                                        ApiRequests::new().challenge_cancel(challenge().nanoid)
                                    }

                                    class="bg-red-500 hover:bg-red-400 transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                                >
                                    Cancel
                                </button>
                                <Show when=move || {
                                    challenge().visibility == ChallengeVisibility::Private
                                        && !single
                                }>
                                    <button
                                        ref=button_ref
                                        on:click=copy
                                        class="bg-blue-500 hover:bg-blue-400 transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                                    >
                                        <Icon icon=Icon::from(AiCopyOutlined)/>
                                    </button>
                                </Show>

                            </div>
                        }
                    }
                >

                    <button
                        on:click=move |_| {
                            log!("User is: {:?}", (auth_context.user) ());
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

                        class="bg-blue-500 hover:bg-blue-400 transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                    >
                        {if challenge().opponent.is_some() { "Accept" } else { "Join" }}

                    </button>
                    {if challenge().opponent.is_some() {
                        view! {
                            <button
                                on:click=move |_| {
                                    log!("User is: {:?}", (auth_context.user) ());
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

                                class="bg-red-500 hover:bg-red-400 transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                            >
                                "Deny"

                            </button>
                        }
                            .into_view()
                    } else {
                        view! {}.into_view()
                    }}

                </Show>
            </td>
        </tr>
    }
}
