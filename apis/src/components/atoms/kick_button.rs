use crate::{
    common::TournamentAction,
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext},
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;
use uuid::Uuid;

#[component]
pub fn KickButton(user_id: Uuid, tournament_id: TournamentId) -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let tournament_id = StoredValue::new(tournament_id);

    let can_kick = move || {
        auth_context
            .user
            .with(|user| user.as_ref().is_some_and(|user| user.id != user_id))
    };

    let kick = move |_| {
        let api = api.get();
        api.tournament(TournamentAction::Kick(tournament_id.get_value(), user_id));
    };

    view! {
        <Show when=can_kick>
            <button
                title=move || t_string!(i18n, tournaments.admin.kick_action).to_string()
                on:click=kick
                class="mx-2 ui-button ui-button-danger ui-button-icon"
            >
                <Icon icon=icondata_ai::AiUserDeleteOutlined attr:class="size-6" />
            </button>
        </Show>
    }
}
