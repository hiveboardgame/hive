use crate::{
    common::{TournamentAction, UserAction},
    components::molecules::{user_row::UserRow, user_search::UserSearch},
    providers::{
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
        TournamentCommonStoreFields,
        TournamentState,
    },
};
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn TournamentOrganizers(
    tournament: TournamentState,
    user_is_organizer_or_admin: Signal<bool>,
) -> impl IntoView {
    let api = expect_context::<ApiRequestsProvider>().0;
    let auth = expect_context::<AuthContext>();
    let tournament_id = StoredValue::new(tournament.tournament_id());
    let active = Signal::derive(move || {
        tournament
            .common
            .lifecycle()
            .with(|lifecycle| lifecycle.finished_at.is_none())
    });
    let organizers = Signal::derive(move || {
        tournament
            .common
            .memberships()
            .with(|memberships| memberships.organizers.clone())
    });
    let invitations = Signal::derive(move || {
        tournament
            .common
            .memberships()
            .with(|memberships| memberships.organizer_invitees.clone())
    });
    let viewer = Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let invited = Signal::derive(move || {
        viewer
            .get()
            .is_some_and(|id| invitations.with(|users| users.iter().any(|user| user.uid == id)))
    });
    let organizing = Signal::derive(move || {
        viewer
            .get()
            .is_some_and(|id| organizers.with(|users| users.iter().any(|user| user.uid == id)))
    });
    let excluded = Signal::derive(move || {
        organizers
            .get()
            .into_iter()
            .chain(invitations.get())
            .map(|user| user.username)
            .collect::<HashSet<_>>()
    });
    view! {
        <section class="space-y-3 ui-setting-group">
            // TODO: i18n once copy is approved.
            <h3 class="font-bold">"Organizers"</h3>
            <For each=move || organizers.get() key=|user| user.uid let:user>
                <UserRow user actions=Vec::new() />
            </For>
            <Show when=move || active.get() && invited.get()>
                // TODO: i18n once copy is approved.
                <p class="text-sm">"You have been invited to organize this tournament."</p>
                <div class="flex gap-2">
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-sm"
                        on:click=move |_| {
                            api.get()
                                .tournament(
                                    TournamentAction::OrganizerAccept(tournament_id.get_value()),
                                );
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Accept organizer invitation"
                    </button>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        on:click=move |_| {
                            api.get()
                                .tournament(
                                    TournamentAction::OrganizerDecline(tournament_id.get_value()),
                                );
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Decline"
                    </button>
                </div>
            </Show>
            <Show when=move || active.get() && user_is_organizer_or_admin.get()>
                // TODO: i18n once copy is approved.
                <UserSearch
                    compact=true
                    placeholder="Invite an organizer"
                    filtered_users=excluded
                    actions=vec![UserAction::InviteOrganizer(tournament_id.get_value())]
                />
                <Show when=move || !invitations.with(Vec::is_empty)>
                    // TODO: i18n once copy is approved.
                    <p class="text-sm font-semibold">"Pending organizer invitations"</p>
                    <For each=move || invitations.get() key=|user| user.uid let:user>
                        {
                            let uid = user.uid;
                            view! {
                                <div class="flex gap-2 items-center">
                                    <div class="flex-1 min-w-0">
                                        <UserRow user actions=Vec::new() />
                                    </div>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-secondary ui-button-sm"
                                        on:click=move |_| {
                                            api.get()
                                                .tournament(
                                                    TournamentAction::OrganizerRetract(
                                                        tournament_id.get_value(),
                                                        uid,
                                                    ),
                                                );
                                        }
                                    >
                                        // TODO: i18n once copy is approved.
                                        "Retract"
                                    </button>
                                </div>
                            }
                        }
                    </For>
                </Show>
                <Show when=move || organizing.get()>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        prop:disabled=move || organizers.with(Vec::len) < 2
                        on:click=move |_| {
                            api.get()
                                .tournament(
                                    TournamentAction::OrganizerLeave(tournament_id.get_value()),
                                );
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Leave as organizer"
                    </button>
                    <Show when=move || organizers.with(Vec::len) < 2>
                        // TODO: i18n once copy is approved.
                        <p class="text-xs text-gray-600 dark:text-gray-300">
                            "Another organizer must accept before you can leave."
                        </p>
                    </Show>
                </Show>
            </Show>
        </section>
    }
}
