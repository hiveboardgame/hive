use crate::{
    common::TournamentAction,
    providers::{ApiRequestsProvider, AuthContext},
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;
use uuid::Uuid;

#[component]
pub fn UninviteButton(user_id: Uuid, tournament_id: TournamentId) -> impl IntoView {
    let user_id = StoredValue::new(user_id);
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth_context = expect_context::<AuthContext>();

    let logged_in_and_not_user = move || {
        auth_context.user.with(|a| {
            a.as_ref()
                .is_some_and(|current_user| current_user.id != user_id.get_value())
        })
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
                title="Remove from tournament"
                on:click=uninvite
                class="p-1 mx-2 text-white rounded transition-transform duration-300 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata_ai::AiUserDeleteOutlined attr:class="size-6" />
            </button>
        </Show>
    }
}
