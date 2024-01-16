use leptos::{html::Dialog, *};
use leptos_icons::{Icon, RiIcon::RiSwordOthersLine};

use crate::{
    components::{
        atoms::{profile_link::ProfileLink, status_indicator::StatusIndicator},
        molecules::modal::Modal,
    },
    pages::challenge_create::ChallengeCreate,
    providers::auth_context::AuthContext,
};

#[component]
pub fn UserRow(username: StoredValue<String>, rating: u64) -> impl IntoView {
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
        <li class="flex p-1 dark:odd:bg-odd-dark dark:even:bg-even-dark odd:bg-odd-light even:bg-even-light items-center justify-between">
            <Modal open=open dialog_el=dialog_el>
                <ChallengeCreate close=close_modal opponent=username()/>
            </Modal>
            <div class="flex items-center w-48 mr-2 justify-between">
                <div class="flex">
                    <StatusIndicator username=username()/>
                    <ProfileLink username=username()/>
                </div>
                <p class="mx-2">{rating}</p>
            </div>
            <Show when=logged_in_and_not_user>
                <button
                    on:click=move |_| open.update(move |b| *b = true)
                    class="mx-2 bg-blue-500 hover:bg-blue-700 transform transition-transform duration-300 active:scale-95 py-2 px-4 rounded"
                >
                    <Icon icon=Icon::from(RiSwordOthersLine)/>
                </button>
            </Show>
        </li>
    }
}
