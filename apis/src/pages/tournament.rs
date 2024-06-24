use crate::common::{TournamentAction, UserAction};
use crate::components::molecules::{
    game_previews::GamePreviews, invite_user::InviteUser, time_row::TimeRow, user_row::UserRow,
};
use crate::providers::{
    navigation_controller::NavigationControllerSignal, tournaments::TournamentStateSignal,
    ApiRequests, AuthContext,
};
use leptos::*;
use leptos_router::use_navigate;
use shared_types::{TimeInfo, TournamentStatus};

const BUTTON_STYLE: &str = "flex gap-1 justify-center items-center px-4 py-2 font-bold text-white rounded bg-button-dawn dark:bg-button-twilight hover:bg-pillbug-teal active:scale-95";

#[component]
pub fn Tournament() -> impl IntoView {
    let navi = expect_context::<NavigationControllerSignal>();
    let tournaments = expect_context::<TournamentStateSignal>();
    let tournament_id = move || navi.tournament_signal.get().tournament_id;
    let current_tournament = move || {
        tournament_id().and_then(|tournament_id| {
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
            current_tournament()
                .map_or(false, |t| t.players.iter().any(|(id, _)| *id == account.id))
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
        if let Some(tournament_id) = tournament_id() {
            if user_is_organizer() {
                let action = TournamentAction::Delete(tournament_id);
                let api = ApiRequests::new();
                api.tournament(action);
                let navigate = use_navigate();
                navigate("/tournaments", Default::default());
            }
        }
    };
    let start = move |_| {
        if let Some(tournament_id) = tournament_id() {
            if user_is_organizer() {
                let action = TournamentAction::Start(tournament_id);
                let api = ApiRequests::new();
                api.tournament(action);
            }
        }
    };
    let leave_or_join = move |_| {
        if let Some(tournament_id) = tournament_id() {
            let action = if user_joined() {
                TournamentAction::Leave(tournament_id)
            } else {
                TournamentAction::Join(tournament_id)
            };
            let api = ApiRequests::new();
            api.tournament(action);
        }
    };

    let display_tournament = move || {
        current_tournament().and_then(|tournament| {
            let time_info = TimeInfo{mode:tournament.time_mode.clone() ,base: tournament.time_base, increment: tournament.time_increment};
            let tournament = store_value(tournament);
            view! {
                <div>"Name: " {tournament().name}</div>
                <div>"Description: " {tournament().description}</div>
                <div>"Scoring: " {tournament().scoring}</div>

                <div>"Seats: " {number_of_players} / {tournament().seats}</div>

                <div>"Rounds: " {tournament().rounds}</div>
                <div class="flex">
                    "Time control: "
                    <TimeRow time_info/>
                </div>

                <div>
                    Organizers
                    <For
                        each=move || { tournament().organizers }

                        key=|users| (users.uid)
                        let:user
                    >
                        <UserRow actions=vec![] user=store_value(user)/>
                    </For>
                </div>
                <Show
                    when=move || tournament().status != TournamentStatus::NotStarted
                    fallback=move || {
                        view! {
                            <div>
                                Players
                                <For
                                    each=move || { tournament().players }

                                    key=|(id, _)| (*id)
                                    let:user
                                >
                                    <UserRow
                                        actions=vec![UserAction::Kick(Box::new(tournament()))]
                                        user=store_value(user.1)
                                    />
                                </For>
                            </div>
                            <div>
                                Invited
                                <For
                                    each=move || { tournament().invitees }
                                    key=|users| (users.uid)
                                    let:user
                                >
                                    <UserRow
                                        actions=vec![
                                            UserAction::Uninvite(tournament().tournament_id.clone()),
                                        ]

                                        user=store_value(user)
                                    />
                                </For>
                            </div>
                            <Show when=user_is_organizer>
                                <InviteUser tournament=tournament()/>
                            </Show>
                            <button class=BUTTON_STYLE on:click=leave_or_join>
                                {join_leave_text}
                            </button>
                            <Show when=user_is_organizer>
                                <button class=BUTTON_STYLE on:click=delete>
                                    {"Delete"}
                                </button>
                            </Show>
                            <Show when=user_is_organizer>
                                <button class=BUTTON_STYLE on:click=start>
                                    {"Start"}
                                </button>
                            </Show>
                        }
                    }
                >

                    <div class="flex flex-col items-center w-full">
                        <For
                            each=move || { tournament().standings.into_iter() }

                            key=|(id, _)| (*id)
                            let:score
                        >
                            {
                                let user_score = store_value(score.1.to_string());
                                let user = store_value(tournament().players.get(&score.0).expect("User in tournament").clone());
                                view! {
                                    <div >
                                        <div class="flex gap-1 items-center">
                                        <UserRow actions=vec![] user/>
                                        {user_score}
                                        </div>
                                    </div>
                                }
                            }

                        </For>
                        Tournament Games:
                        <div class="flex flex-wrap">
                            <GamePreviews games=Callback::new(move |_| (tournament().games))/>
                        </div>
                    </div>
                </Show>
            }
            .into()
        })
    };
    view! { <div class="flex flex-col pt-10 w-full">{display_tournament}</div> }
}
