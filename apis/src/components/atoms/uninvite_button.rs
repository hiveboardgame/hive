use crate::{
    common::TournamentAction,
    providers::{ApiRequestsProvider, AuthContext},
    responses::UserResponse,
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;

#[component]
pub fn UninviteButton(user: UserResponse, tournament_id: TournamentId) -> impl IntoView {
    let user = StoredValue::new(user);
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();

    let logged_in_and_not_user = move || {
        auth_context
            .user
            .get()
            .is_some_and(|current_user| current_user.id != user.get_value().uid)
    };
    let tournament_id = StoredValue::new(tournament_id);

    let uninvite = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::InvitationRetract(
            tournament_id.get_value(),
            user.get_value().uid,
        ));
    };

    view! {
        <Show when=logged_in_and_not_user>
            <button
                title="Remove from tournament"
                on:click=uninvite
                class="p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata::AiUserDeleteOutlined attr:class="w-6 h-6" />
            </button>
        </Show>
    }
}
