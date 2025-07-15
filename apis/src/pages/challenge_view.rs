use crate::components::molecules::challenge_row::ChallengeRow;
use crate::functions::{challenges::get::get_challenge, hostname::hostname_and_port};
use crate::providers::AuthContext;
use leptos::either::Either;
use leptos::{html, prelude::*};
use leptos_icons::*;
use leptos_router::hooks::use_params;
use leptos_router::params::Params;
use leptos_use::use_window;
use shared_types::ChallengeId;

#[derive(Params, PartialEq, Eq)]
struct ChallengeParams {
    nanoid: String,
}

#[component]
pub fn ChallengeView() -> impl IntoView {
    let params = use_params::<ChallengeParams>();
    let auth_context = expect_context::<AuthContext>();
    let nanoid = move || {
        params.with(|params| {
            params
                .as_ref()
                .map(|params| params.nanoid.clone())
                .unwrap_or_default()
        })
    };
    let challenge_id = move || ChallengeId(nanoid());
    let challenge = OnceResource::new(get_challenge(challenge_id()));

    let challenge_address = move || format!("{}/challenge/{}", hostname_and_port(), nanoid());
    let button_ref = NodeRef::<html::Button>::new();
    let copy = move |_| {
        let clipboard = use_window()
            .as_ref()
            .expect("window to exist in challenge_view")
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
    let uid = move || auth_context.user.with(|a| a.as_ref().map(|user| user.id));
    view! {
        <div class="flex flex-col items-center pt-20 mx-auto">
            <Transition>
                {move || {
                    challenge
                        .get()
                        .map(|data| match data {
                            Err(_) => {
                                Either::Left(view! { <pre>"Challenge doesn't seem to exist"</pre> })
                            }
                            Ok(challenge) => {
                                let user = auth_context.user;
                                Either::Right(
                                    view! {
                                        <Show when=move || {
                                            user.with(|a| {
                                                a.as_ref()
                                                    .is_some_and(|user| user.id == challenge.challenger.uid)
                                            })
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
                                                    node_ref=button_ref
                                                    on:click=copy
                                                    class="px-1 py-1 m-1 font-bold text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95 focus:outline-none focus:shadow-outline"
                                                >
                                                    <Icon icon=icondata::AiCopyOutlined attr:class="w-6 h-6" />
                                                </button>
                                            </div>
                                            <p>
                                                "The first person to come to this URL will play with you."
                                            </p>
                                        </Show>
                                        <ChallengeRow challenge=challenge single=true uid=uid() />
                                    },
                                )
                            }
                        })
                }}

            </Transition>
        </div>
    }
}
