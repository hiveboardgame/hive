use crate::{
    common::UserAction, components::molecules::user_search::UserSearch,
    responses::TournamentResponse,
};
use leptos::prelude::*;
use std::collections::HashSet;

#[component]
pub fn InviteUser(tournament: TournamentResponse) -> impl IntoView {
    let filtered_users: HashSet<String> = tournament
        .players
        .values()
        .map(|player| &player.username)
        .chain(tournament.invitees.iter().map(|invitee| &invitee.username))
        .cloned()
        .collect();

    view! {
        <div class="flex flex-col justify-center items-center">
            <UserSearch
                placeholder="Invite player".to_string()
                filtered_users=filtered_users
                actions=vec![UserAction::Invite(tournament.tournament_id.clone())]
            />
        </div>
    }
}
