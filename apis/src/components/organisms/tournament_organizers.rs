use crate::{
    common::{tournament_path_matches, TournamentAction, UserAction},
    components::{
        molecules::{user_identity::UserIdentity, user_search::UserSearch},
        organisms::tournament_admin::MEMBERSHIP_ROW_CLASS,
    },
    providers::{
        ApiRequestsProvider,
        AuthContext,
        AuthIdentity,
        TournamentCommonStoreFields,
        TournamentState,
    },
};
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_location};
use std::collections::HashSet;

#[component]
pub fn TournamentOrganizers(
    tournament: TournamentState,
    user_is_organizer_or_admin: Signal<bool>,
    #[prop(optional)] managing: bool,
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
    let pathname = use_location().pathname;
    let send = Callback::new(move |action| {
        let id = tournament_id.get_value();
        if active.get_untracked()
            && tournament_path_matches(&pathname.get_untracked(), &id)
            && tournament.common.lifecycle().get_untracked().tournament_id == id
        {
            api.get().tournament(action);
        }
    });
    view! {
        <section class=if managing { "space-y-4" } else { "space-y-3 ui-setting-group" }>
            <Show when=move || !managing>
                <div class="flex flex-wrap gap-2 justify-between items-center">
                    // TODO: i18n once copy is approved.
                    <h3 class="font-bold">"Organizers"</h3>
                    <Show when=move || active.get() && user_is_organizer_or_admin.get()>
                        // TODO: i18n once copy is approved.
                        <A
                            href=move || {
                                format!(
                                    "/tournament/{}/manage/people#organizers",
                                    tournament_id.get_value().0,
                                )
                            }
                            attr:class="text-xs ui-text-link"
                        >
                            "Manage organizers"
                        </A>
                    </Show>
                </div>
                <div class="space-y-2">
                    <For each=move || organizers.get() key=|user| user.uid let:user>
                        <UserIdentity user link_class="truncate" />
                    </For>
                </div>
            </Show>
            <Show when=move || active.get() && invited.get()>
                // TODO: i18n once copy is approved.
                <p class="text-sm">"You have been invited to organize this tournament."</p>
                <div class="flex flex-wrap gap-2">
                    <button
                        type="button"
                        class="ui-button ui-button-primary ui-button-sm"
                        on:click=move |_| {
                            send.run(TournamentAction::OrganizerAccept(tournament_id.get_value()))
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Accept organizer invitation"
                    </button>
                    <button
                        type="button"
                        class="ui-button ui-button-secondary ui-button-sm"
                        on:click=move |_| {
                            send.run(TournamentAction::OrganizerDecline(tournament_id.get_value()))
                        }
                    >
                        // TODO: i18n once copy is approved.
                        "Decline"
                    </button>
                </div>
            </Show>
            <Show when=move || managing && active.get() && user_is_organizer_or_admin.get()>
                // TODO: i18n once copy is approved.
                <h2 class="text-lg font-bold">
                    {move || format!("Organizers ({})", organizers.with(Vec::len))}
                </h2>
                <div class="w-full min-w-0">
                    // TODO: i18n once copy is approved.
                    <UserSearch
                        compact=true
                        placeholder="Invite an organizer"
                        filtered_users=excluded
                        actions=vec![UserAction::InviteOrganizer(tournament_id.get_value())]
                    />
                </div>
                // TODO: i18n once copy is approved.
                <p class="text-sm text-gray-600 dark:text-gray-300">
                    "Organizers share tournament management. Invite them as players separately if they will also compete."
                </p>
                <div class="space-y-2">
                    <For each=move || organizers.get() key=|user| user.uid let:user>
                        {
                            let uid = user.uid;
                            view! {
                                <div class=MEMBERSHIP_ROW_CLASS>
                                    <div class="min-w-0">
                                        <UserIdentity user link_class="truncate" />
                                    </div>
                                    <Show when=move || viewer.get() == Some(uid)>
                                        <button
                                            type="button"
                                            class="shrink-0 ui-button ui-button-ghost ui-button-sm"
                                            prop:disabled=move || organizers.with(Vec::len) < 2
                                            on:click=move |_| {
                                                if !user_is_organizer_or_admin.get_untracked() {
                                                    return;
                                                }
                                                if viewer.get_untracked() == Some(uid)
                                                    && organizers.with_untracked(Vec::len) >= 2
                                                {
                                                    send.run(
                                                        TournamentAction::OrganizerLeave(tournament_id.get_value()),
                                                    );
                                                }
                                            }
                                        >
                                            // TODO: i18n once copy is approved.
                                            "Leave role"
                                        </button>
                                    </Show>
                                </div>
                            }
                        }
                    </For>
                </div>
                <Show when=move || !invitations.with(Vec::is_empty)>
                    <div class="space-y-2">
                        // TODO: i18n once copy is approved.
                        <h3 class="text-sm font-semibold">
                            {move || {
                                format!("Pending invitations ({})", invitations.with(Vec::len))
                            }}
                        </h3>
                        <For each=move || invitations.get() key=|user| user.uid let:user>
                            {
                                let uid = user.uid;
                                view! {
                                    <div class=MEMBERSHIP_ROW_CLASS>
                                        <div class="min-w-0">
                                            <UserIdentity user link_class="truncate" />
                                        </div>
                                        <button
                                            type="button"
                                            class="shrink-0 ui-button ui-button-ghost ui-button-sm"
                                            on:click=move |_| {
                                                if user_is_organizer_or_admin.get_untracked() {
                                                    send.run(
                                                        TournamentAction::OrganizerRetract(
                                                            tournament_id.get_value(),
                                                            uid,
                                                        ),
                                                    );
                                                }
                                            }
                                        >
                                            // TODO: i18n once copy is approved.
                                            "Cancel invite"
                                        </button>
                                    </div>
                                }
                            }
                        </For>
                    </div>
                </Show>
                <Show when=move || organizing.get() && organizers.with(Vec::len) < 2>
                    // TODO: i18n once copy is approved.
                    <p class="text-xs text-gray-600 dark:text-gray-300">
                        "Another organizer must accept before you can leave."
                    </p>
                </Show>
            </Show>
        </section>
    }
}
