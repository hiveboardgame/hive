use crate::{
    common::TournamentAction,
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext, AuthIdentity},
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentId;
use uuid::Uuid;

#[component]
pub fn InviteButton(
    user_id: Uuid,
    tournament_id: TournamentId,
    #[prop(optional)] organizer: bool,
) -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let logged_in_and_not_user = move || {
        auth_context
            .identity
            .get()
            .and_then(AuthIdentity::user_id)
            .is_some_and(|current_user_id| current_user_id != user_id)
    };

    let tournament_id = StoredValue::new(tournament_id);

    let invite = move |_| {
        let api = api.get();
        api.tournament(if organizer {
            TournamentAction::OrganizerInvite(tournament_id.get_value(), user_id)
        } else {
            TournamentAction::InvitationCreate(tournament_id.get_value(), user_id)
        });
    };

    view! {
        <Show when=logged_in_and_not_user>
            <button
                title=move || {
                    if organizer {
                        String::from("Invite as organizer")
                    } else {
                        t_string!(i18n, tournaments.admin.invite_action).to_string()
                    }
                }
                on:click=invite
                class="mx-2 ui-button ui-button-primary ui-button-icon"
            >
                <Icon icon=icondata_ai::AiUserAddOutlined attr:class="size-6" />
            </button>
        </Show>
    }
}
