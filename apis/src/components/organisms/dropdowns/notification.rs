use crate::components::molecules::{
    challenge_row::ChallengeRow, hamburger::Hamburger,
    tournament_invitation_notification::TournamentInvitationNotification,
    tournament_status_notification::TournamentStatusNotification,
};
use crate::providers::challenges::ChallengeStateSignal;
use crate::providers::tournaments::TournamentStateContext;
use crate::providers::NotificationContext;
use crate::responses::TournamentResponse;
use leptos::prelude::*;
use leptos::text_prop::TextProp;
use leptos_icons::*;

//TODO: Put style back into the icon
#[component]
pub fn NotificationDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let notifications_context = Signal::derive(move || expect_context::<NotificationContext>());
    let challenges = expect_context::<ChallengeStateSignal>();
    let tournaments = expect_context::<TournamentStateContext>();
    let has_notifications = move || !notifications_context().is_empty();
    let icon_style = move || {
        if has_notifications() {
            "w-4 h-4 fill-ladybug-red"
        } else {
            "w-4 h-4"
        }
    };
    let each_tournament = move || {
        notifications_context()
            .tournament_invitations
            .get()
            .iter()
            .map(move |id| {
                tournaments
                    .full
                    .get()
                    .tournaments
                    .get(id)
                    .expect("Tournament exists")
                    .clone()
            })
            .collect::<Vec<TournamentResponse>>()
    };
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="h-full p-2 transform transition-transform duration-300 active:scale-95 whitespace-nowrap block"
            dropdown_style="mr-1 items-center xs:mt-0 mt-1 flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md p-2 right-0"
            content=view! {<span class=icon_style> <Icon icon=icondata::IoNotifications attr:class="w-full h-full"/> </span> }
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
                            challenge=StoredValue::new(
                                challenges
                                    .signal
                                    .get_untracked()
                                    .challenges
                                    .get(&challenge_id)
                                    .expect("Challenge exists")
                                    .clone(),
                            )
                            single=false
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
                                .full
                                .get_untracked()
                                .tournaments
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
                                .full
                                .get_untracked()
                                .tournaments
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
