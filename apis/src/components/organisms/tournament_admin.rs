use crate::components::molecules::invite_user::InviteUser;
use crate::components::molecules::user_row::UserRow;
use crate::{common::UserAction, responses::TournamentResponse};
use leptos::prelude::*;

#[component]
pub fn TournamentAdminControls(
    user_is_organizer: bool,
    tournament: StoredValue<TournamentResponse>,
) -> impl IntoView {
    let user_kick = move || {
        if user_is_organizer {
            vec![UserAction::Kick(Box::new(tournament.get_value()))]
        } else {
            vec![]
        }
    };
    let user_uninvite = move || {
        if user_is_organizer {
            vec![UserAction::Uninvite(
                tournament.with_value(|t| t.tournament_id.clone()),
            )]
        } else {
            vec![]
        }
    };
    view! {
        <div class="flex flex-col items-center px-1 w-72">
            <Show when=move || tournament.with_value(|t| !t.players.is_empty())>
                <p class="font-bold">Players</p>
                <For
                    each=move || { tournament.with_value(|t| t.players.clone()) }

                    key=|(id, _)| *id
                    let:user
                >
                    <UserRow actions=user_kick() user=user.1 />
                </For>
            </Show>
        </div>
        <div class="flex flex-col items-center px-1 w-72">
            <Show when=move || tournament.with_value(|t| !t.invitees.is_empty())>
                <p class="font-bold">Invitees</p>
                <For
                    each=move || { tournament.with_value(|t| t.invitees.clone()) }
                    key=|users| users.uid
                    let:user
                >
                    <UserRow actions=user_uninvite() user />
                </For>
            </Show>
            <Show when=move || user_is_organizer>
                <p class="font-bold">Invite players</p>
                <InviteUser tournament />
            </Show>
        </div>
    }
}
