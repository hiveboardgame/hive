use crate::{
    i18n::*,
    providers::{AuthContext, AuthIdentity, DirectChallengeOpener},
};
use leptos::prelude::*;
use leptos_icons::*;
use uuid::Uuid;

#[component]
pub fn DirectChallengeButton(
    user_id: Uuid,
    opponent: String,
    #[prop(optional)] show_label: bool,
    #[prop(optional, into)] disabled: Signal<bool>,
) -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let direct_challenge = expect_context::<DirectChallengeOpener>();
    let opponent = StoredValue::new(opponent);
    let logged_in_and_not_user = move || {
        auth_context
            .identity
            .get()
            .and_then(AuthIdentity::user_id)
            .is_some_and(|current_user_id| current_user_id != user_id)
    };

    view! {
        <Show when=logged_in_and_not_user>
            <button
                title=move || t_string!(i18n, game.challenge_to_a_game)
                prop:disabled=disabled
                on:click=move |_| direct_challenge.open(opponent.get_value())
                class=if show_label {
                    "ui-button ui-button-primary ui-button-sm gap-2"
                } else {
                    "mx-2 ui-button ui-button-primary ui-button-icon"
                }
            >
                <Icon icon=icondata_ri::RiSwordOthersLine attr:class="size-6" />
                <Show when=move || show_label>
                    <span>{t!(i18n, profile.about.challenge)}</span>
                </Show>
            </button>
        </Show>
    }
}
