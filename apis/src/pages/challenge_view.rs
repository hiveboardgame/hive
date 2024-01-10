use crate::components::molecules::challenge_row::ChallengeRow;
use crate::functions::{challenges::get::get_challenge_by_nanoid, hostname::hostname_and_port};
use crate::providers::auth_context::AuthContext;
use leptos::*;
use leptos_icons::{AiIcon::AiCopyOutlined, Icon};
use leptos_router::*;
use leptos_use::use_window;

#[derive(Params, PartialEq, Eq)]
struct ChallengeParams {
    nanoid: String,
}

#[component]
pub fn ChallengeView() -> impl IntoView {
    let params = use_params::<ChallengeParams>();
    let auth_context = expect_context::<AuthContext>();
    // id: || -> usize
    let nanoid = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };

    let challenge = Resource::once(move || get_challenge_by_nanoid(nanoid()));
    let challenge_address = move || format!("{}/challenge/{}", hostname_and_port(), nanoid());
    let button_ref = create_node_ref::<html::Button>();
    let copy = move |_| {
        let clipboard = use_window()
            .as_ref()
            .expect("window to exist in challenge_view")
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

    view! {
        <div class="pt-20 mx-auto flex flex-col items-center">
            <Transition>
                {move || {
                    challenge()
                        .map(|data| match data {
                            Err(_) => {
                                view! { <pre>"Challenge doesn't seem to exist"</pre> }.into_view()
                            }
                            Ok(challenge) => {
                                let user = move || match (auth_context.user)() {
                                    Some(Ok(Some(user))) => Some(user),
                                    _ => None,
                                };
                                view! {
                                    <Show when=move || {
                                        if user().is_some() {
                                            user().expect("there to be a user").id
                                                == challenge.challenger.uid
                                        } else {
                                            false
                                        }
                                    }>
                                        <p>"To invite someone to play, give this URL:"</p>
                                        <div class="flex">
                                            <input
                                                id="challenge_link"
                                                type="text"
                                                class="w-[50ch]"
                                                value=challenge_address
                                                readonly
                                            />
                                            <button
                                                title="Copy link"
                                                ref=button_ref
                                                on:click=copy
                                                class="bg-blue-500 hover:bg-blue-400 duration-300 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                                            >
                                                <Icon icon=Icon::from(AiCopyOutlined)/>
                                            </button>
                                        </div>
                                        <p>
                                            "The first person to come to this URL will play with you."
                                        </p>
                                    </Show>
                                    <ChallengeRow challenge=store_value(challenge) single=true/>
                                }
                                    .into_view()
                            }
                        })
                }}

            </Transition>
        </div>
    }
}
