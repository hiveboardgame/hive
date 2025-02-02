use crate::{
    common::TournamentAction,
    providers::{ApiRequests, AuthContext},
    responses::UserResponse,
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;

#[component]
pub fn UninviteButton(
    user: UserResponse,
    tournament_id: TournamentId,
) -> impl IntoView {
    let user = StoredValue::new(user);
    let auth_context = expect_context::<AuthContext>();

    let logged_in_and_not_user = move || {
        if let Some(Ok(current_user)) = auth_context.user.get() {
            current_user.id != user.get_value().uid
        } else {
            false
        }
    };

    let tournament_id = StoredValue::new(tournament_id);

    let uninvite = move |_| {
        let api = ApiRequests::new();
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
