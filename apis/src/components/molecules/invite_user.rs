use crate::{
    common::UserAction,
    components::molecules::user_search::UserSearch,
    i18n::*,
    providers::{TournamentCommonStoreFields, TournamentState},
};
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn InviteUser(tournament: TournamentState) -> impl IntoView {
    let i18n = use_i18n();
    let filtered_users = Signal::derive(move || {
        let memberships = tournament.common.memberships().get();
        memberships
            .players
            .values()
            .map(|player| &player.username)
            .chain(memberships.invitees.iter().map(|invitee| &invitee.username))
            .chain(
                memberships
                    .declined_invitees
                    .iter()
                    .map(|invitee| &invitee.username),
            )
            .cloned()
            .collect::<HashSet<_>>()
    });
    let tournament_id = tournament.tournament_id();

    view! {
        <div class="flex flex-col flex-1 justify-center w-full min-w-0">
            <UserSearch
                compact=true
                placeholder=move || {
                    t_string!(i18n, tournaments.admin.invite_placeholder).to_string()
                }
                filtered_users=filtered_users
                actions=vec![UserAction::Invite(tournament_id)]
            />
        </div>
    }
}
