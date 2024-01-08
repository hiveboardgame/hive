use crate::common::challenge_action::ChallengeVisibility;
use crate::providers::api_requests::ApiRequests;
use crate::{
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
    BiIcon::BiInfiniteRegular,
    BsIcon::{BsHexagon, BsHexagonFill, BsHexagonHalf},
    Icon,
};
use leptos_router::*;
use leptos_use::use_window;
use std::str::FromStr;
use shared_types::time_mode::TimeMode;

#[component]
pub fn DisplayChallenge(challenge: StoredValue<ChallengeResponse>, single: bool) -> impl IntoView {
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

    view! {
        <tr class="dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light text-center items-center">
            <td class=td_class>{icon}</td>
            <td class=td_class>
                <ProfileLink username=challenge().challenger.username/>
            </td>
            <td class=td_class>{challenge().challenger.rating}</td>
            <td class=td_class>
                {if challenge().game_type == "Base" { "🚫" } else { "🦟🐞💊" }}
            </td>
            <td class=td_class>

                {move || {
                    let time_mode = TimeMode::from_str(&challenge().time_mode)
                        .expect("Valid TimeMode");
                    match time_mode {
                        TimeMode::Untimed => {
                            view! { <Icon icon=Icon::from(BiInfiniteRegular)/> }
                        }
                        TimeMode::RealTime => {
                            view! {
                                <p>
                                    "Realtime: " {challenge().time_base.expect("Time exists")/60} "m"
                                    " + " {challenge().time_increment.expect("Increment exists")}
                                    "s"
                                </p>
                            }
                                .into_view()
                        }
                        TimeMode::Correspondence => {
                            view! {
                                <p>
                                    "Correspondence: Days/move "
                                    {challenge().time_base.expect("Time exists")}
                                </p>
                            }
                                .into_view()
                        }
                    }
                }}

            </td>
            <td class=td_class>
                <span class="font-bold">{if challenge().rated { "RATED" } else { "CASUAL" }}</span>
            </td>
            <td class=td_class>
                <Show
                    when=move || {
                        let user = move || match (auth_context.user)() {
                            Some(Ok(Some(user))) => Some(user),
                            _ => None,
                        };
                        if user().is_some() {
                            user().expect("there to be a user").id != challenge().challenger.uid
                        } else {
                            true
                        }
                    }

                    fallback=move || {
                        view! {
                            <div class="flex">
                                <button
                                    on:click=move |_| {
                                        ApiRequests::new().challenge_cancel(challenge().nanoid)
                                    }

                                    class="bg-red-500 hover:bg-red-400 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                                >
                                    Cancel
                                </button>
                                <Show when=move || {
                                    challenge().visibility != ChallengeVisibility::Public && !single
                                }>
                                    <button
                                        ref=button_ref
                                        on:click=copy
                                        class="bg-blue-500 hover:bg-blue-400 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
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
                                    let games = game_state.signal.get().next_games;
                                    game_state.full_reset();
                                    game_state.signal.update(|s| s.next_games = games);
                                    ApiRequests::new().challenge_accept(challenge().nanoid);
                                }
                                _ => {
                                    let navigate = use_navigate();
                                    navigate("/login", Default::default());
                                }
                            }
                        }

                        class="bg-blue-500 hover:bg-blue-400 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                    >
                        Join
                    </button>
                </Show>
            </td>
        </tr>
    }
}
