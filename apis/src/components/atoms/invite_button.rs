use crate::{
    common::TournamentAction,
    providers::{ApiRequestsProvider, AuthContext},
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
        auth_context.user.with(|a| {
            a.as_ref()
                .is_some_and(|current_user| current_user.id != user_id)
        })
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
                class="p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata_ai::AiUserAddOutlined attr:class="w-6 h-6" />
            </button>
        </Show>
    }
}
