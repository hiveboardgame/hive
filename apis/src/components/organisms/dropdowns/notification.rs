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
    let notifications_context = Signal::derive(move || expect_context::<NotificationContext>());
    let challenges = expect_context::<ChallengeStateSignal>();
    let has_notifications = move || !notifications_context().is_empty();
    let icon_style = move || {
        if has_notifications() {
            "w-4 h-4 fill-ladybug-red"
        } else {
            "w-4 h-4"
        }
    };
    let tournaments = LocalResource::new(move || async move {
        let vec = get_all_abstract(TournamentSortOrder::CreatedAtDesc)
            .await
            .unwrap_or_default();
        let mut map = HashMap::new();
        for t in vec {
            map.insert(t.tournament_id.clone(), t);
        }
        map
    });
    let each_tournament = move || {
        notifications_context()
            .tournament_invitations
            .get()
            .iter()
            .filter_map(move |id| {
                tournaments
                    .get()
                    .map(|t| t.get(id).expect("Tournament exists").clone())
            })
            .collect::<Vec<TournamentAbstractResponse>>()
    };
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
                <For
                    each=move || notifications_context().challenges.get()
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

                <For each=each_tournament key=|t| { (t.id, t.seats) } let:tournament>
                    <div on:click=onclick_close>
                        <TournamentInvitationNotification tournament=RwSignal::new(
                            tournament.clone(),
                        ) />
                    </div>
                </For>

                <For
                    each=move || notifications_context().tournament_started.get()
                    key=|c| { c.clone() }
                    let:tournament_id
                >
                    <div on:click=onclick_close>
                        <TournamentStatusNotification tournament=StoredValue::new(
                            tournaments
                                .get()
                                .expect("Loaded")
                                .get(&tournament_id)
                                .expect("Tournament exists")
                                .clone(),
                        ) />
                    </div>
                </For>

                <For
                    each=move || notifications_context().tournament_finished.get()
                    key=|c| { c.clone() }
                    let:tournament_id
                >
                    <div on:click=onclick_close>
                        <TournamentStatusNotification tournament=StoredValue::new(
                            tournaments
                                .get()
                                .expect("Loaded")
                                .get(&tournament_id)
                                .expect("Tournament exists")
                                .clone(),
                        ) />
                    </div>
                </For>
            </Show>
        </Hamburger>
    }
}
