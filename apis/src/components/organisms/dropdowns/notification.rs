use crate::components::molecules::{
    challenge_row::ChallengeRow, hamburger::Hamburger,
    tournament_invitation_notification::TournamentInvitationNotification,
    tournament_status_notification::TournamentStatusNotification,
};
use crate::functions::tournaments::get_all_abstract;
use crate::providers::challenges::ChallengeStateSignal;
use crate::providers::{AuthContext, NotificationContext};
use crate::responses::TournamentAbstractResponse;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::TournamentSortOrder;
use std::collections::HashMap;

#[component]
pub fn NotificationDropdown() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let uid = move || auth_context.user.get().map(|user| user.id);
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let notifications_context = StoredValue::new(expect_context::<NotificationContext>());
    let challenges = expect_context::<ChallengeStateSignal>();
    let has_notifications = move || !notifications_context.get_value().is_empty();
    let icon_style = move || {
        if has_notifications() {
            "w-4 h-4 fill-ladybug-red"
        } else {
            "w-4 h-4"
        }
    };

    //TODO: Getting all tournaments each time when has_notifications changes is not good longterm
    let tournaments_resource = Resource::new(has_notifications, move |_| {
        get_all_abstract(TournamentSortOrder::CreatedAtDesc)
    });

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="h-full p-2 transform transition-transform duration-300 active:scale-95 whitespace-nowrap block"
            dropdown_style="mr-1 items-center xs:mt-0 mt-1 flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md p-2 right-0"
            content=view! { <Icon icon=icondata::IoNotifications attr:class=icon_style /> }
            id="Notifications"
        >
            <Show
                when=has_notifications
                fallback=|| {
                    view! { "No notifications right now" }
                }
            >
                <Transition>
                    {move || Suspend::new(async move {
                        let tournaments:StoredValue<HashMap<_,_>> = StoredValue::new(
                            tournaments_resource
                                .await
                                .unwrap_or_default()
                                .into_iter()
                                .map(|t| (t.tournament_id.clone(), t))
                                .collect(),
                        );
                        let each_tournament = move || {
                            notifications_context
                                .get_value()
                                .tournament_invitations
                                .get()
                                .iter()
                                .filter_map(move |id| {
                                    tournaments.get_value().get(id).cloned()
                                })
                                .collect::<Vec<TournamentAbstractResponse>>()
                        };

                        view! {
                            <For
                                each=move || notifications_context.get_value().challenges.get()
                                key=|c| { c.clone() }
                                let:challenge_id
                            >
                                <div on:click=onclick_close>
                                    <ChallengeRow
                                        challenge=challenges
                                            .signal
                                            .get_untracked()
                                            .challenges
                                            .get(&challenge_id)
                                            .expect("Challenge exists")
                                            .clone()
                                        single=false
                                        uid=uid()
                                    />
                                </div>
                            </For>

                            <For
                                each=each_tournament
                                key=|t| { (t.id, t.players, t.seats) }
                                let:tournament
                            >
                                <div on:click=onclick_close>
                                    <TournamentInvitationNotification tournament />
                                </div>
                            </For>

                            <For
                                each=move || {
                                    notifications_context.get_value().tournament_started.get()
                                }
                                key=|c| { c.clone() }
                                let:tournament_id
                            >
                                <div on:click=onclick_close>
                                    <TournamentStatusNotification
                                        tournament_id=tournament_id.clone()
                                        tournament_name=
                                         tournaments.get_value()
                                         .get(&tournament_id)
                                         .expect("tournament exists")
                                         .name
                                         .clone()
                                        finished=false
                                    />
                                </div>
                            </For>

                            <For
                                each=move || {
                                    notifications_context.get_value().tournament_finished.get()
                                }
                                key=|c| { c.clone() }
                                let:tournament_id
                            >
                                <div on:click=onclick_close>
                                    <TournamentStatusNotification
                                        tournament_id=tournament_id.clone()
                                        tournament_name=
                                         tournaments.get_value()
                                         .get(&tournament_id)
                                         .expect("tournament exists")
                                         .name
                                         .clone()
                                        finished=true
                                    />
                                </div>
                            </For>
                        }
                    })}
                </Transition>
            </Show>
        </Hamburger>
    }
}
