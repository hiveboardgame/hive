use crate::components::molecules::{
    challenge_row::ChallengeRow, hamburger::Hamburger,
    tournament_invitation_row::TournamentInvitationRow,
};
use crate::providers::challenges::ChallengeStateSignal;
use crate::providers::tournaments::TournamentStateSignal;
use crate::providers::NotificationContext;
use leptos::*;
use leptos_icons::*;

#[component]
pub fn NotificationDropdown() -> impl IntoView {
    let hamburger_show = create_rw_signal(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let notifications_context = store_value(expect_context::<NotificationContext>());
    let challenges = expect_context::<ChallengeStateSignal>();
    let tournaments = expect_context::<TournamentStateSignal>();
    let has_notifications = move || !notifications_context().is_empty();
    let icon_style = TextProp::from(move || {
        if has_notifications() {
            "w-4 h-4 fill-ladybug-red"
        } else {
            "w-4 h-4"
        }
    });
    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="h-full p-2 transform transition-transform duration-300 active:scale-95 whitespace-nowrap block"
            dropdown_style="mr-1 items-center xs:mt-0 mt-1 flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md p-2 right-0"
            content=view! { <Icon icon=icondata::IoNotifications class=icon_style/> }
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
                            challenge=store_value(
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

                <For
                    each=move || notifications_context().tournaments.get()
                    key=|t| { t.clone() }
                    let:tournament_id
                >
                    <div on:click=onclick_close>
                        <TournamentInvitationRow
                            tournament=store_value(
                                tournaments
                                    .signal
                                    .get_untracked()
                                    .tournaments
                                    .get(&tournament_id)
                                    .expect("Tournament exists")
                                    .clone(),
                            )
                        />
                    </div>
                </For>
            </Show>
        </Hamburger>
    }
}
