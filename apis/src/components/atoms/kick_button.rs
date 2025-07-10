use crate::{
    common::TournamentAction,
    providers::{ApiRequestsProvider, AuthContext},
    responses::TournamentResponse,
};
use leptos::prelude::*;
use leptos_icons::*;
use uuid::Uuid;

#[component]
pub fn KickButton(user_id: Uuid, tournament: TournamentResponse) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let tournament = StoredValue::new(tournament);

    let is_organizer = move || {
        if let Some(current_user) = auth_context.user.get() {
            current_user.id != user_id
                && tournament
                    .get_value()
                    .organizers
                    .iter()
                    .any(|o| o.uid == current_user.id)
        } else {
            false
        }
    };

    let kick = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::Kick(
            tournament.get_value().tournament_id,
            user_id,
        ));
    };

    view! {
        <Show when=is_organizer>
            <button
                title="Remove from tournament"
                on:click=kick
                class="p-1 mx-2 text-white rounded transition-transform duration-300 transform bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata::AiUserDeleteOutlined attr:class="w-6 h-6" />
            </button>
        </Show>
    }
}
