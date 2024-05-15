use crate::{
    components::molecules::modal::Modal, pages::challenge_create::ChallengeCreate,
    providers::AuthContext, responses::UserResponse,
};
use leptos::{html::Dialog, *};
use leptos_icons::*;

#[component]
pub fn DirectChallengeButton(user: StoredValue<UserResponse>) -> impl IntoView {
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
        if let Some(Ok(Some(current_user))) = (auth_context.user)() {
            current_user.id != user().uid
        } else {
            false
        }
    };

    view! {
        <Modal open=open dialog_el=dialog_el>
            <ChallengeCreate close=close_modal opponent=user().username/>
        </Modal>
        <Show when=logged_in_and_not_user>
            <button
                title="Challenge to a game"
                on:click=move |_| open.update(move |b| *b = true)
                class="p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata::RiSwordOthersLine class="w-6 h-6"/>
            </button>
        </Show>
    }
}
