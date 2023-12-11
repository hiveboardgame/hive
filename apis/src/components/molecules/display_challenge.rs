use crate::{
    functions::{
        challenges::{
            accept::AcceptChallenge, challenge_response::ChallengeResponse, delete::DeleteChallenge,
        },
        hostname::hostname_and_port,
    },
    providers::{auth_context::AuthContext, color_scheme::ColorScheme},
};
use hive_lib::color::ColorChoice;
use leptos::*;
use leptos_icons::{
    AiIcon::AiCopyOutlined,
    BiIcon::BiInfiniteRegular,
    BsIcon::{BsHexagon, BsHexagonFill, BsHexagonHalf},
    Icon,
};
use leptos_router::*;
use leptos_use::use_window;

#[component]
pub fn DisplayChallenge(challenge: StoredValue<ChallengeResponse>, single: bool) -> impl IntoView {
    let accept_challenge = create_server_action::<AcceptChallenge>();
    let delete_challenge = create_server_action::<DeleteChallenge>();
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
        <tr class="dark:odd:bg-slate-700 dark:even:bg-slate-800 odd:bg-stone-300 even:bg-stone-100 text-center items-center">
            <td class=td_class>{icon}</td>
            <td class=td_class>{challenge().challenger.username}</td>
            <td class=td_class>{challenge().challenger.rating}</td>
            <td class=td_class>
                {if challenge().game_type == "Base" { "🚫" } else { "🦟🐞💊" }}
            </td>
            <td class=td_class>
                <Icon icon=Icon::from(BiInfiniteRegular) class="h-full w-full"/>
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
                                <ActionForm action=delete_challenge class="flex">
                                    <input
                                        type="hidden"
                                        name="id"
                                        value=challenge().id.to_string()
                                    />
                                    <input
                                        type="submit"
                                        value="Cancel"
                                        class="grow bg-red-600 hover:bg-red-500 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                                    />
                                </ActionForm>
                                <Show when=move || !challenge().public && !single>
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

                    <ActionForm action=accept_challenge>
                        <input type="hidden" name="nanoid" value=challenge().nanoid/>
                        <input
                            type="submit"
                            value="Join"
                            class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                        />
                    </ActionForm>
                </Show>
            </td>
        </tr>
    }
}
