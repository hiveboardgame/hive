use crate::{
    common::TournamentAction, providers::{AuthContext, ClientApi}, responses::TournamentResponse,
};
use leptos::prelude::*;
use leptos_icons::*;
use uuid::Uuid;

#[component]
pub fn KickButton(user_id: Uuid, tournament: TournamentResponse) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let client_api = expect_context::<ClientApi>();
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
        let api = client_api;
        let tournament_id = tournament.with_value(|t| t.tournament_id.clone());
        api.tournament(TournamentAction::Kick(tournament_id, user_id));
    };

    view! {
        <Show when=is_organizer>
            <button
                title="Remove from tournament"
                on:click=kick
                class="p-1 mx-2 text-white rounded transition-transform duration-300 bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal dark:hover:bg-pillbug-teal active:scale-95"
            >
                <Icon icon=icondata_ai::AiUserDeleteOutlined attr:class="size-6" />
            </button>
        </Show>
    }
}
