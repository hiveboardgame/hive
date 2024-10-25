use crate::components::molecules::invite_user::InviteUser;
use crate::components::molecules::user_row::UserRow;
use crate::{common::UserAction, responses::TournamentResponse};
use leptos::*;

#[component]
pub fn TournamentAdminControls(
    user_is_organizer: bool,
    tournament: TournamentResponse,
) -> impl IntoView {
    let tournament = store_value(tournament);
    let user_kick = move || {
        if user_is_organizer {
            vec![UserAction::Kick(Box::new(tournament()))]
        } else {
            vec![]
        }
    };
    let user_uninvite = move || {
        if user_is_organizer {
            vec![UserAction::Uninvite(tournament().tournament_id)]
        } else {
            vec![]
        }
    };
    view! {
        <div class="flex flex-col items-center px-1 w-72">
            <Show when=move || !tournament().players.is_empty()>
                <p class="font-bold">Players</p>
                <For
                    each=move || { tournament().players }

                    key=|(id, _)| (*id)
                    let:user
                >
                    <UserRow actions=user_kick() user=store_value(user.1) />
                </For>
            </Show>
        </div>
        <div class="flex flex-col items-center px-1 w-72">
            <Show when=move || !tournament().invitees.is_empty()>
                <p class="font-bold">Invitees</p>
                <For each=move || { tournament().invitees } key=|users| (users.uid) let:user>
                    <UserRow actions=user_uninvite() user=store_value(user) />
                </For>
            </Show>
            <Show when=move || user_is_organizer>
                <p class="font-bold">Invite players</p>
                <InviteUser tournament=tournament() />
            </Show>
        </div>
    }
}
