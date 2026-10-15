use crate::{
    common::TournamentAction,
    i18n::*,
    providers::ApiRequestsProvider,
    responses::TournamentAbstractResponse,
};
use leptos::prelude::*;

#[component]
pub fn TournamentOrganizerInvitationNotification(
    tournament: TournamentAbstractResponse,
) -> impl IntoView {
    let i18n = use_i18n();
    let api = expect_context::<ApiRequestsProvider>().0;
    let id = StoredValue::new(tournament.tournament_id);
    view! {
        <div class="ui-notification-item">
            <a class="flex-1 min-w-0" href=format!("/tournament/{}", id.get_value())>

                <div class="ui-notification-label">
                    {t!(i18n, tournaments.ui.organizer_invitation)}
                </div>
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

                    {t!(i18n, common.accept)}
                </button>
                <button
                    type="button"
                    class="ui-button ui-button-secondary ui-button-sm"
                    on:click=move |_| {
                        api.get().tournament(TournamentAction::OrganizerDecline(id.get_value()));
                    }
                >

                    {t!(i18n, common.decline)}
                </button>
            </div>
        </div>
    }
}
