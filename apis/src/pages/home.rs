use crate::providers::auth_context::AuthContext;
use crate::{
    components::{
        molecules::modal::Modal,
        organisms::{challenges::Challenges, players::PlayersView},
    },
    pages::challenge_create::ChallengeCreate,
};
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
    let logged_in = move || matches!((auth_context.user)(), Some(Ok(Some(_))));

    view! {
        <div class="pt-16 flex flex-col justify-start md:justify-center items-center w-full overflow-x-clip">
            <Show when=logged_in>
                <button
                    class="m-5 grow md:grow-0 whitespace-nowrap bg-blue-500 hover:bg-blue-700 transform transition-transform duration-300 active:scale-95 text-white font-bold py-2 px-4 rounded"
                    on:click=move |_| open.update(move |b| *b = true)
                >
                    Create New Game
                </button>
            </Show>
            <div class="flex flex-col md:flex-row justify-center">
                <Modal open=open dialog_el=dialog_el>
                    <ChallengeCreate close=close_modal/>
                </Modal>
                <div class="flex">
                    <div class="flex flex-col w-full">
                        <Challenges/>
                    </div>
                </div>
                <PlayersView/>
            </div>
        </div>
    }
}
