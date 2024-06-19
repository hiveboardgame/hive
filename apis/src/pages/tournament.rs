use crate::common::{TournamentAction, UserAction};
use crate::components::molecules::invite_user::InviteUser;
use crate::components::molecules::time_row::TimeRow;
use crate::components::molecules::user_row::UserRow;
use crate::providers::{
    navigation_controller::NavigationControllerSignal, tournaments::TournamentStateSignal,
    ApiRequests, AuthContext,
};
use leptos::*;
use leptos_router::use_navigate;

const BUTTON_STYLE: &str = "flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95";

#[component]
pub fn Tournament() -> impl IntoView {
    let navi = expect_context::<NavigationControllerSignal>();
    let tournaments = expect_context::<TournamentStateSignal>();
    let nanoid = move || navi.tournament_signal.get().nanoid;
    let current_tournament = move || {
        nanoid().and_then(|tournament_id| {
            tournaments
                .signal
                .get()
                .tournaments
                .get(&tournament_id)
                .cloned()
        })
    };
    let auth_context = expect_context::<AuthContext>();
    let account = move || match (auth_context.user)() {
        Some(Ok(Some(account))) => Some(account),
        _ => None,
    };
    let number_of_players = move || current_tournament().map_or(0, |t| t.players.len());
    let user_joined = move || {
        if let Some(account) = account() {
            current_tournament().map_or(false, |t| t.players.iter().any(|p| p.uid == account.id))
        } else {
            false
        }
    };
    let user_is_organizer = move || {
        if let Some(account) = account() {
            current_tournament().map_or(false, |t| t.organizers.iter().any(|p| p.uid == account.id))
        } else {
            false
        }
    };
    let join_leave_text = move || {
        if user_joined() {
            "Leave"
        } else {
            "Join"
        }
    };
    let delete = move |_| {
        if let Some(nanoid) = nanoid() {
            if user_is_organizer() {
                let action = TournamentAction::Delete(nanoid);
                let api = ApiRequests::new();
                api.tournament(action);
                let navigate = use_navigate();
                navigate("/tournaments", Default::default());
            }
        }
    };
    let leave_or_join = move |_| {
        if let Some(nanoid) = nanoid() {
            let action = if user_joined() {
                TournamentAction::Leave(nanoid)
            } else {
                TournamentAction::Join(nanoid)
            };
            let api = ApiRequests::new();
            api.tournament(action);
        }
    };
    let display_tournament = move || {
        current_tournament().and_then(|tournament| {
            let tournament = store_value(tournament);
            view! {
                <div>{tournament().name}</div>
                <div>{tournament().description}</div>
                <div>{tournament().scoring}</div>
                <div>
                    Organizers
                    <For
                        each=move || { tournament().organizers.clone() }

                        key=|users| (users.uid)
                        let:user
                    >
                        <UserRow actions=vec![] user=store_value(user)/>
                    </For>
                </div>
                <div>
                    Players
                    <For
                        each=move || { tournament().players.clone() }

                        key=|users| (users.uid)
                        let:user
                    >
                        <UserRow actions=vec![] user=store_value(user)/>
                    </For>
                </div>
                <div>
                    Invited
                    <For
                        each=move || { tournament().invitees.clone() }
                        key=|users| (users.uid)
                        let:user
                    >
                        <UserRow
                            actions=vec![UserAction::Invite(tournament().nanoid.clone())]
                            user=store_value(user)
                        />
                    </For>
                </div>
                <InviteUser tournament=tournament().nanoid/>
                Seats
                <div>{number_of_players} / {tournament().seats}</div>
                Rounds
                <div>{tournament().rounds}</div>
                <TimeRow
                    time_mode=tournament().time_mode
                    time_base=tournament().time_base
                    increment=tournament().time_increment
                />
                <button class=BUTTON_STYLE on:click=leave_or_join>
                    {join_leave_text}
                </button>
                <Show when=user_is_organizer>
                    <button class=BUTTON_STYLE on:click=delete>
                        {"Delete"}
                    </button>
                </Show>
            }
            .into()
        })
    };
    view! { <div class="flex flex-col pt-10">{display_tournament}</div> }
}
