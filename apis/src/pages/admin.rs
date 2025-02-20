use crate::{components::organisms::chat::ChatWindow, providers::AuthContext};
use leptos::prelude::*;
use shared_types::SimpleDestination;

#[component]
pub fn Admin() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    view! {
        <div class="pt-20">
            <Show when=move || {
                if let Some(Ok(account)) = auth_context.user.get() {
                    account.user.admin
                } else {
                    false
                }
            }>
                <ChatWindow destination=SimpleDestination::Global />
            </Show>
        </div>
    }
}
