use crate::{
    common::TournamentAction,
    providers::{ApiRequests, AuthContext},
    responses::UserResponse,
};
use leptos::*;
use leptos_icons::*;
use uuid::Uuid;

#[component]
pub fn InviteButton(user: StoredValue<UserResponse>, tournament_nanoid: String) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();

    let logged_in_and_not_user = move || {
        if let Some(Ok(Some(current_user))) = (auth_context.user)() {
            current_user.id != user().uid
        } else {
            false
        }
    };
    let tournament_id = store_value(tournament_nanoid);

    let invite = move |_| {
        let api = ApiRequests::new();
        api.tournament(TournamentAction::InvitationCreate(
            tournament_id(),
            user().uid,
        ));
    };

    view! {
        <Show when=logged_in_and_not_user>
            <button
                title="Invite to tournament"
                on:click=invite
                class="p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata::AiUserAddOutlined class="w-6 h-6"/>
            </button>
        </Show>
    }
}
