use crate::{
    components::molecules::modal::Modal, pages::challenge_create::ChallengeCreate,
    providers::auth_context::AuthContext,
};
use leptos::{html::Dialog, *};
use leptos_icons::{Icon, RiIcon::RiSwordOthersLine};

#[component]
pub fn DirectChallenge(username: StoredValue<String>) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let open = create_rw_signal(false);
    let dialog_el = create_node_ref::<Dialog>();
    let close_modal = Callback::new(move |()| {
        dialog_el
            .get_untracked()
            .expect("dialog to have been created")
            .close();
    });

    let logged_in_and_not_user = move || {
        if let Some(Ok(Some(user))) = (auth_context.user)() {
            user.username != username()
        } else {
            false
        }
    };

    view! {
        <Modal open=open dialog_el=dialog_el>
            <ChallengeCreate close=close_modal opponent=username()/>
        </Modal>
        <Show when=logged_in_and_not_user>
            <button
                title="Challenge to a game"
                on:click=move |_| open.update(move |b| *b = true)
                class="mx-2 bg-blue-500 hover:bg-blue-700 transform transition-transform duration-300 active:scale-95 py-2 px-4 rounded"
            >
                <Icon icon=Icon::from(RiSwordOthersLine)/>
            </button>
        </Show>
    }
}
