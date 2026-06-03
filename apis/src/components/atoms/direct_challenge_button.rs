use crate::providers::{AuthContext, DirectChallengeOpener};
use leptos::prelude::*;
use leptos_icons::*;
use uuid::Uuid;

#[component]
pub fn DirectChallengeButton(
    user_id: Uuid,
    opponent: String,
    #[prop(optional, into)] disabled: Signal<bool>,
) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let direct_challenge = expect_context::<DirectChallengeOpener>();
    let opponent = StoredValue::new(opponent);
    let logged_in_and_not_user = move || {
        auth_context.user.with(|a| {
            a.as_ref()
                .is_some_and(|current_user| current_user.id != user_id)
        })
    };

    view! {
        <Show when=logged_in_and_not_user>
            <button
                title="Challenge to a game"
                prop:disabled=disabled
                on:click=move |_| direct_challenge.open(opponent.get_value())
                class="p-1 mx-2 text-white rounded transition-transform duration-300 transform active:scale-95 disabled:opacity-25 disabled:cursor-not-allowed bg-button-dawn dark:bg-button-twilight dark:hover:bg-pillbug-teal hover:bg-pillbug-teal"
            >
                <Icon icon=icondata_ri::RiSwordOthersLine attr:class="size-6" />
            </button>
        </Show>
    }
}
