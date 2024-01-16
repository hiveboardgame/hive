use crate::components::{molecules::modal::Modal, organisms::challenges::Challenges};
use crate::pages::challenge_create::ChallengeCreate;
use crate::providers::auth_context::AuthContext;
use leptos::{html::Dialog, *};

#[component]
pub fn Home() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let open = create_rw_signal(false);
    let dialog_el = create_node_ref::<Dialog>();
    let close_modal = Callback::new(move |()| {
        dialog_el
            .get_untracked()
            .expect("dialog to have been created")
            .close();
    });

    let logged_in = move || {
        if let Some(Ok(Some(_))) = (auth_context.user)() {
            true
        } else {
            false
        }
    };

    view! {
        <div class="pt-16 flex flex-col justify-center place-items-center">
            <a
                href="/players"
                class="m-5 grow md:grow-0 max-w-fit whitespace-nowrap duration-300 bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded"
            >
                Leaderboard and online players
            </a>
            <div class="pt-5 flex flex-col md:flex-row justify-center">
                <Modal open=open dialog_el=dialog_el>
                    <ChallengeCreate close=close_modal/>
                </Modal>
                <div class="flex justify-center">
                    <div class="flex flex-col max-w-fit w-full">
                        <Challenges/>
                    </div>
                </div>
                <Show when=logged_in>
                    <div class="flex md:flex-col">
                        <button
                            class="m-5 md:mt-14 grow md:grow-0 whitespace-nowrap bg-blue-500 hover:bg-blue-700 transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded"
                            on:click=move |_| open.update(move |b| *b = true)
                        >
                            Create New Game
                        </button>
                    </div>
                </Show>
            </div>
        </div>
    }
}
