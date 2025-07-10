use crate::components::molecules::{
    challenge_row::ChallengeRow,
    hamburger::Hamburger,
    schedule_notification::{AcceptanceNotification, ProposalNotification},
    tournament_invitation_notification::TournamentInvitationNotification,
    tournament_status_notification::TournamentStatusNotification,
};
use crate::functions::tournaments::get_abstracts_by_ids;
use crate::providers::challenges::ChallengeStateSignal;
use crate::providers::{AuthContext, NotificationContext, SchedulesContext};
use crate::responses::ScheduleResponse;
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::GameId;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

fn get_schedule_details(
    schedules: RwSignal<HashMap<GameId, HashMap<Uuid, ScheduleResponse>>>,
    schedule_id: Uuid,
) -> Option<ScheduleResponse> {
    schedules.with(|schedules| {
        for (_, game_schedules) in schedules.iter() {
            if let Some(schedule) = game_schedules.get(&schedule_id) {
                return Some(schedule.clone());
            }
        }
        None
    })
}

#[component]
pub fn NotificationDropdown() -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let uid = move || auth_context.user.get().map(|user| user.id);
    let hamburger_show = RwSignal::new(false);
    let onclick_close = move |_| hamburger_show.update(|b| *b = false);
    let notifications_context = StoredValue::new(expect_context::<NotificationContext>());
    let challenges = expect_context::<ChallengeStateSignal>();
    let schedules_context = expect_context::<SchedulesContext>();
    let has_notifications = move || !notifications_context.get_value().is_empty();

    let icon_style = move || {
        if has_notifications() {
            "w-4 h-4 fill-ladybug-red"
        } else {
            "w-4 h-4"
        }
    };

    let has_tournament_notifications = move || {
        let ctx = notifications_context.get_value();
        !ctx.tournament_invitations.get().is_empty()
            || !ctx.tournament_started.get().is_empty()
            || !ctx.tournament_finished.get().is_empty()
    };

    let tournaments_resource = Resource::new(has_tournament_notifications, move |_| {
        let notifications_ctx = notifications_context.get_value();
        let mut tournament_ids = HashSet::new();
        tournament_ids.extend(
            notifications_ctx
                .tournament_invitations
                .get()
                .iter()
                .cloned(),
        );
        tournament_ids.extend(notifications_ctx.tournament_started.get().iter().cloned());
        tournament_ids.extend(notifications_ctx.tournament_finished.get().iter().cloned());
        get_abstracts_by_ids(tournament_ids)
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
                <For
                    each=move || notifications_context.get_value().challenges.get()
                    key=|c| c.clone()
                    let:challenge_id
                >
                    <div on:click=onclick_close>
                        <table class="border-collapse table-auto">
                            <tbody>
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
                                    attr:class="p-2 rounded dark:bg-header-twilight bg-odd-light text-sm"
                                />
                            </tbody>
                        </table>
                    </div>
                </For>

                <For
                    each=move || notifications_context.get_value().schedule_proposals.get()
                    key=|id| *id
                    let:schedule_id
                >
                    {move || {
                        get_schedule_details(schedules_context.own, schedule_id)
                            .map(|schedule| {
                                view! {
                                    <div on:click=onclick_close>
                                        <ProposalNotification
                                            schedule_id=schedule.id
                                            proposer_username=schedule.proposer_username
                                            tournament_id=schedule.tournament_id
                                            start_time=schedule.start_t
                                        />
                                    </div>
                                }
                            })
                    }}
                </For>

                <For
                    each=move || notifications_context.get_value().schedule_acceptances.get()
                    key=|id| *id
                    let:schedule_id
                >
                    {move || {
                        get_schedule_details(schedules_context.own, schedule_id)
                            .map(|schedule| {
                                view! {
                                    <div on:click=onclick_close>
                                        <AcceptanceNotification
                                            schedule_id=schedule.id
                                            accepter_username=schedule.opponent_username
                                            tournament_id=schedule.tournament_id
                                            start_time=schedule.start_t
                                        />
                                    </div>
                                }
                            })
                    }}
                </For>

                <Show when=has_tournament_notifications>
                    <Transition fallback=|| {
                        view! { <div>"Loading tournaments..."</div> }
                    }>
                        {move || Suspend::new(async move {
                            let tournaments: HashMap<_, _> = tournaments_resource
                                .await
                                .unwrap_or_default()
                                .into_iter()
                                .map(|t| (t.tournament_id.clone(), t))
                                .collect();
                            let tournaments = StoredValue::new(tournaments);
                            let ctx = notifications_context.get_value();

                            view! {
                                <For
                                    each=move || ctx.tournament_invitations.get()
                                    key=|id| id.clone()
                                    let:tournament_id
                                >
                                    {move || {
                                        tournaments
                                            .get_value()
                                            .get(&tournament_id)
                                            .map(|tournament| {
                                                view! {
                                                    <div on:click=onclick_close>
                                                        <TournamentInvitationNotification tournament=tournament
                                                            .clone() />
                                                    </div>
                                                }
                                            })
                                    }}
                                </For>

                                <For
                                    each=move || ctx.tournament_started.get()
                                    key=|id| id.clone()
                                    let:tournament_id
                                >
                                    {move || {
                                        tournaments
                                            .get_value()
                                            .get(&tournament_id)
                                            .map(|tournament| {
                                                view! {
                                                    <div on:click=onclick_close>
                                                        <TournamentStatusNotification
                                                            tournament_id=tournament_id.clone()
                                                            tournament_name=tournament.name.clone()
                                                            finished=false
                                                        />
                                                    </div>
                                                }
                                            })
                                    }}
                                </For>

                                <For
                                    each=move || ctx.tournament_finished.get()
                                    key=|id| id.clone()
                                    let:tournament_id
                                >
                                    {move || {
                                        tournaments
                                            .get_value()
                                            .get(&tournament_id)
                                            .map(|tournament| {
                                                view! {
                                                    <div on:click=onclick_close>
                                                        <TournamentStatusNotification
                                                            tournament_id=tournament_id.clone()
                                                            tournament_name=tournament.name.clone()
                                                            finished=true
                                                        />
                                                    </div>
                                                }
                                            })
                                    }}
                                </For>
                            }
                        })}
                    </Transition>
                </Show>
            </Show>
        </Hamburger>
    }
}
