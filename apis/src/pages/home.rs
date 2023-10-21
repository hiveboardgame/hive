use crate::components::{molecules::modal::Modal, organisms::lobby::Lobby};
use crate::pages::challenge_create::ChallengeCreate;
use crate::functions::challenges::challenge_response::ChallengeResponse;
use crate::providers::auth_context::AuthContext;
use leptos::{html::Dialog, *};
use leptos_query::use_query_client;

#[component]
pub fn Home(#[prop(optional)] extend_tw_classes: &'static str) -> impl IntoView {
    let client = use_query_client();
    let open = create_rw_signal(false);
    let auth_context = expect_context::<AuthContext>();
    let dialog_el = create_node_ref::<Dialog>();
    let onsubmit = move || {
        client.invalidate_query::<(), Result<Vec<ChallengeResponse>, ServerFnError>>(());
        dialog_el
            .get_untracked()
            .expect("dialog to have been created")
            .close();
    };
    let stored_on_submit = store_value(onsubmit);
    view! {
        <div class=format!("{extend_tw_classes}")>
            <Transition>
                {move || {
                    let user = move || match auth_context.user.get() {
                        Some(Ok(Some(user))) => Some(user),
                        _ => None,
                    };
                    view! {
                        <Show when=move || user().is_some()>
                            <button
                                class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline m-1"
                                on:click=move |_| open.update(move |b| *b = true)
                            >
                                Create New Game
                            </button>
                            <Modal open=open dialog_el=dialog_el>
                                <ChallengeCreate on:submit=move |_| stored_on_submit()()/>
                            </Modal>
                        </Show>
                    }
                }}

            </Transition>
            <Lobby/>
        </div>
    }
}

