use crate::{
    components::molecules::modal::Modal, pages::challenge_bot::ChallengeBot, providers::AuthContext,
};
use leptos::{html::Dialog, prelude::*};
use leptos_router::hooks::use_navigate;

#[component]
pub fn PlayBot() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let dialog_el = NodeRef::<Dialog>::new();

    view! {
        <Modal dialog_el>
            <ChallengeBot />
        </Modal>

        <button
            title="Play vs bot"
            on:click=move |_| {
                if auth_context.user.with(|a| a.is_some()) {
                    if let Some(dialog_el) = dialog_el.get() {
                        let _ = dialog_el.show_modal();
                    }
                } else {
                    let navigate = use_navigate();
                    navigate("/login", Default::default());
                }
            }
            class="flex gap-1 justify-center items-center px-4 py-2 font-bold text-white whitespace-nowrap rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
        >
            Play bot
        </button>
    }
}
