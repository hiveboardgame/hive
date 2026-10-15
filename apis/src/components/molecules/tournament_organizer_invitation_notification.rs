use crate::{
    common::TournamentAction,
    providers::ApiRequestsProvider,
    responses::TournamentAbstractResponse,
};
use leptos::prelude::*;

#[component]
pub fn TournamentOrganizerInvitationNotification(
    tournament: TournamentAbstractResponse,
) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0;
    let id = StoredValue::new(tournament.tournament_id);
    view! {
        <div class="ui-notification-item">
            <a class="flex-1 min-w-0" href=format!("/tournament/{}", id.get_value())>
                // TODO: i18n once copy is approved.
                <div class="ui-notification-label">"Organizer invitation"</div>
                <div class="ui-notification-title">{tournament.name}</div>
            </a>
            <div class="ui-notification-actions">
                <button
                    type="button"
                    class="ui-button ui-button-primary ui-button-sm"
                    on:click=move |_| {
                        api.get().tournament(TournamentAction::OrganizerAccept(id.get_value()));
                    }
                >
                    // TODO: i18n once copy is approved.
                    "Accept"
                </button>
                <button
                    type="button"
                    class="ui-button ui-button-secondary ui-button-sm"
                    on:click=move |_| {
                        api.get().tournament(TournamentAction::OrganizerDecline(id.get_value()));
                    }
                >
                    // TODO: i18n once copy is approved.
                    "Decline"
                </button>
            </div>
        </div>
    }
}
