use crate::{
    components::molecules::modal::Modal,
    pages::challenge_create::ChallengeCreate,
    providers::DirectChallengeState,
};
use leptos::prelude::*;

#[component]
pub fn DirectChallengeModal(state: DirectChallengeState) -> impl IntoView {
    view! {
        <Modal dialog_el=state.dialog_el>
            <ChallengeCreate opponent=state.target />
        </Modal>
    }
}
