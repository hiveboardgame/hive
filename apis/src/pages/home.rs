use crate::components::{molecules::modal::Modal, organisms::challenges::Challenges};
use crate::pages::challenge_create::ChallengeCreate;
use crate::providers::auth_context::AuthContext;
use crate::providers::challenges::ChallengeStateSignal;
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
    let close_modal = Callback::new(move |()| {
        dialog_el
            .get_untracked()
            .expect("dialog to have been created")
            .close();
    });

    let source_signal: RwSignal<Option<Uuid>> = create_rw_signal(None);
    let challenge_state = expect_context::<ChallengeStateSignal>();
    let own_challenges = move || challenge_state.signal.get().own;
    let public_challenges = move || challenge_state.signal.get().public;
    let button_color = move || {
        if show_public() {
            ("bg-slate-400", "bg-inherit")
        } else {
            ("bg-inherit", "bg-slate-400")
        }
    };

    view! {
        <div class="pt-16 flex flex-col justify-center place-items-center">
            <a
                href="/players"
                class="m-5 grow md:grow-0 max-w-fit whitespace-nowrap bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
            >
                Leaderboard and online players
            </a>
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
                        view! { <Challenges challenges=public_challenges()/> }
                    };
                    let own = move || {
                        view! { <Challenges challenges=own_challenges()/> }
                    };
                    view! {
                        <div class="pt-5 flex flex-col md:flex-row justify-center">
                            <Show when=move || user().is_some() fallback=challenge>
                                <Modal open=open dialog_el=dialog_el>
                                    <ChallengeCreate close=close_modal/>
                                </Modal>
                                <div class="flex justify-center">
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
                                        class="m-5 md:mt-20 grow md:grow-0 whitespace-nowrap bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
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
        </div>
    }
}
