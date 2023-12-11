use crate::components::{molecules::modal::Modal, organisms::lobby::Lobby};
use crate::functions::challenges::get_challenges::{get_own_challenges, get_public_challenges};
use crate::pages::challenge_create::ChallengeCreate;
use crate::providers::auth_context::AuthContext;
use leptos::{html::Dialog, *};
use uuid::Uuid;

#[component]
pub fn Home() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    // showing public games or own challenges
    let show_public = create_rw_signal(true);
    // if modal is open
    let open = create_rw_signal(false);
    let dialog_el = create_node_ref::<Dialog>();
    let on_submit = move |_| {
        dialog_el
            .get_untracked()
            .expect("dialog to have been created")
            .close();
    };

    let source_signal: RwSignal<Option<Uuid>> = create_rw_signal(None);
    //blocking resource leads to warning but good performance, local resource leads to no warning but fe is beaving slightly worse
    let challenges = create_local_resource(source_signal, move |s| get_public_challenges(s));
    //this one is fine as blocking
    let own_challenges = create_blocking_resource(source_signal, move |s| get_own_challenges(s));
    let button_color = move || {
        if show_public() {
            ("bg-slate-400", "bg-inherit")
        } else {
            ("bg-inherit", "bg-slate-400")
        }
    };

    view! {
        <Transition>
            {move || {
                let user = move || match (auth_context.user)() {
                    Some(Ok(Some(user))) => {
                        source_signal.update(move |v| *v = Some(user.id));
                        Some(user)
                    }
                    _ => {
                        source_signal.update(move |v| *v = None);
                        None
                    }
                };
                let challenge = move || {
                    if let Some(Ok(challenges)) = challenges() {
                        view! { <Lobby challenges=store_value(challenges)/> }
                    } else {
                        ().into_view()
                    }
                };
                let own = move || {
                    if let Some(Ok(Some(own_challenges))) = own_challenges() {
                        view! { <Lobby challenges=store_value(own_challenges)/> }
                    } else {
                        ().into_view()
                    }
                };
                view! {
                    <div class="flex flex-col md:flex-row justify-center col-span-full min-h-fit">
                        <Show when=move || user().is_some() fallback=challenge>
                            <Modal open=open dialog_el=dialog_el>
                                <ChallengeCreate on:submit=on_submit/>
                            </Modal>
                            <div class="max-h-[80vh] flex justify-center">
                                <div class="flex flex-col max-w-fit w-full">
                                    <div class="flex justify-between">
                                        <button
                                            class=move || {
                                                format!("grow hover:bg-blue-300 {}", button_color().0)
                                            }

                                            on:click=move |_| { show_public.update(|b| *b = true) }
                                        >
                                            Lobby
                                        </button>
                                        <button
                                            class=move || {
                                                format!("grow hover:bg-blue-300 {}", button_color().1)
                                            }

                                            on:click=move |_| { show_public.update(|b| *b = false) }
                                        >
                                            My Challenges
                                        </button>
                                    </div>
                                    <Show when=show_public fallback=own>
                                        {challenge}
                                    </Show>
                                </div>

                            </div>
                            <div class="flex md:flex-col">
                                <button
                                    class="m-5 md:mt-20 grow md:grow-0 whitespace-nowrap bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline"
                                    on:click=move |_| open.update(move |b| *b = true)
                                >
                                    Create New Game
                                </button>
                            </div>
                        </Show>

                    </div>
                }
            }}

        </Transition>
    }
}
