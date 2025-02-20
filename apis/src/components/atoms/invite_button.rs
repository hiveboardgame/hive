use crate::{
    common::TournamentAction,
    providers::{ApiRequestsProvider, AuthContext},
    responses::UserResponse,
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;

#[component]
pub fn InviteButton(user: UserResponse, tournament_id: TournamentId) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let logged_in_and_not_user = move || {
        if let Some(Ok(current_user)) = auth_context.user.get() {
            current_user.id != user.uid
        } else {
            false
        }
    };

    let tournament_id = StoredValue::new(tournament_id);

    let invite = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::InvitationCreate(
            tournament_id.get_value(),
            user.uid,
        ));
    };

    view! {
        <Show when=logged_in_and_not_user>
            <button
                title="Invite to tournament"
                on:click=invite
                class="p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata::AiUserAddOutlined attr:class="w-6 h-6" />
            </button>
        </Show>
    }
}
