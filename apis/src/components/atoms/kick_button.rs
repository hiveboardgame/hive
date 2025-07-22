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
        if let Some(current_id) = auth_context.user.with(|a| a.as_ref().map(|u| u.id)) {
            current_id != user_id
                && tournament.with_value(|t| t.organizers.iter().any(|o| o.uid == current_id))
        } else {
            false
        }
    };

    let kick = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::Kick(
            tournament.with_value(|t| t.tournament_id.clone()),
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
                <Icon icon=icondata_ai::AiUserDeleteOutlined attr:class="w-6 h-6" />
            </button>
        </Show>
    }
}
