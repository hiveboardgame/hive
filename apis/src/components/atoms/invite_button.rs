use crate::{
    common::TournamentAction,
    providers::{ApiRequestsProvider, AuthContext, AuthIdentity},
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;
use uuid::Uuid;

#[component]
pub fn InviteButton(user_id: Uuid, tournament_id: TournamentId) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let logged_in_and_not_user = move || {
        matches!(
            auth_context.identity.get(),
            Some(AuthIdentity::User(current_user_id)) if current_user_id != user_id
        )
    };

    let tournament_id = StoredValue::new(tournament_id);

    let invite = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::InvitationCreate(
            tournament_id.get_value(),
            user_id,
        ));
    };

    view! {
        <Show when=logged_in_and_not_user>
            <button
                title="Invite to tournament"
                on:click=invite
                class="mx-2 ui-button ui-button-primary ui-button-icon"
            >
                <Icon icon=icondata_ai::AiUserAddOutlined attr:class="size-6" />
            </button>
        </Show>
    }
}
