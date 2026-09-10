use crate::{
    common::TournamentAction,
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext, AuthIdentity},
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;
use uuid::Uuid;

#[component]
pub fn UninviteButton(user_id: Uuid, tournament_id: TournamentId) -> impl IntoView {
    let i18n = use_i18n();
    let user_id = StoredValue::new(user_id);
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();

    let logged_in_and_not_user = move || {
        auth_context
            .identity
            .get()
            .and_then(AuthIdentity::user_id)
            .is_some_and(|current_user_id| current_user_id != user_id.get_value())
    };
    let tournament_id = StoredValue::new(tournament_id);

    let uninvite = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::InvitationRetract(
            tournament_id.get_value(),
            user_id.get_value(),
        ));
    };

    view! {
        <Show when=logged_in_and_not_user>
            <button
                title=move || t_string!(i18n, tournaments.admin.uninvite_action).to_string()
                on:click=uninvite
                class="mx-2 ui-button ui-button-danger ui-button-icon"
            >
                <Icon icon=icondata_ai::AiUserDeleteOutlined attr:class="size-6" />
            </button>
        </Show>
    }
}
