use crate::{
    components::molecules::modal::Modal, pages::challenge_create::ChallengeCreate,
    providers::AuthContext,
};
use leptos::{html::Dialog, prelude::*};
use leptos_icons::*;
use uuid::Uuid;

#[component]
pub fn DirectChallengeButton(user_id: Uuid, opponent: String) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let dialog_el = NodeRef::<Dialog>::new();
    let logged_in_and_not_user = move || {
        auth_context
            .user
            .get()
            .is_some_and(|current_user| current_user.id != user_id)
    };

    view! {
        <Modal dialog_el>
            <ChallengeCreate opponent />
        </Modal>
        <Show when=logged_in_and_not_user>
            <button
                title="Challenge to a game"
                on:click=move |_| {
                    if let Some(dialog_el) = dialog_el.get() {
                        let _ = dialog_el.show_modal();
                    }
                }
                class="p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata::RiSwordOthersLine attr:class="w-6 h-6" />
            </button>
        </Show>
    }
}
