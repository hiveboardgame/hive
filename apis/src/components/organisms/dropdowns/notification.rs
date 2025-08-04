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
    let uid = move || auth_context.user.with(|a| a.as_ref().map(|user| user.id));
    let hamburger_show = RwSignal::new(false);
    let notifications_context = expect_context::<NotificationContext>();
    let challenges = expect_context::<ChallengeStateSignal>();
    let schedules_context = expect_context::<SchedulesContext>();
    let has_notifications = move || !notifications_context.is_empty();

    let icon_style = move || {
        if has_notifications() {
            "w-4 h-4 fill-ladybug-red"
        } else {
            "w-4 h-4"
        }
    };

    let has_tournament_notifications = move || {
        !notifications_context
            .tournament_invitations
            .with(|v| v.is_empty())
            || !notifications_context
                .tournament_started
                .with(|v| v.is_empty())
            || !notifications_context
                .tournament_finished
                .with(|v| v.is_empty())
    };

    let tournaments_resource = Resource::new(has_tournament_notifications, move |_| {
        let mut tournament_ids = HashSet::new();
        tournament_ids.extend(
            notifications_context
                .tournament_invitations
                .get()
                .iter()
                .cloned(),
        );
        tournament_ids.extend(
            notifications_context
                .tournament_started
                .get()
                .iter()
                .cloned(),
        );
        tournament_ids.extend(
            notifications_context
                .tournament_finished
                .get()
                .iter()
                .cloned(),
        );
        get_abstracts_by_ids(tournament_ids)
    });

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="h-full p-2 transform transition-transform duration-300 active:scale-95 whitespace-nowrap block"
            dropdown_style="mr-1 items-center xs:mt-0 mt-1 flex flex-col items-stretch absolute bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md p-2 right-0 z-50"
            content=view! { <Icon icon=icondata_io::IoNotifications attr:class=icon_style /> }
            id="Notifications"
        >
            <Show
                when=has_notifications
                fallback=|| {
                    view! { "No notifications right now" }
                }
            >
                <For
                    each=move || notifications_context.challenges.get()
                    key=|c| c.clone()
                    let:challenge_id
                >
                    <div>
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
                                    attr:class="p-2 text-sm rounded dark:bg-header-twilight bg-odd-light"
                                />
                            </tbody>
                        </table>
                    </div>
                </For>

                <For
                    each=move || notifications_context.schedule_proposals.get()
                    key=|id| *id
                    let:schedule_id
                >
                    {move || {
                        get_schedule_details(schedules_context.own, schedule_id)
                            .map(|schedule| {
                                view! {
                                    <div>
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
                    each=move || notifications_context.schedule_acceptances.get()
                    key=|id| *id
                    let:schedule_id
                >
                    {move || {
                        get_schedule_details(schedules_context.own, schedule_id)
                            .map(|schedule| {
                                view! {
                                    <div>
                                        <AcceptanceNotification
                                            tournament_name=schedule.tournament_name
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

                            view! {
                                <For
                                    each=move || notifications_context.tournament_invitations.get()
                                    key=|id| id.clone()
                                    let:tournament_id
                                >
                                    {move || {
                                        tournaments
                                            .get_value()
                                            .get(&tournament_id)
                                            .map(|tournament| {
                                                view! {
                                                    <div>
                                                        <TournamentInvitationNotification tournament=tournament
                                                            .clone() />
                                                    </div>
                                                }
                                            })
                                    }}
                                </For>

                                <For
                                    each=move || notifications_context.tournament_started.get()
                                    key=|id| id.clone()
                                    let:tournament_id
                                >
                                    {move || {
                                        tournaments
                                            .get_value()
                                            .get(&tournament_id)
                                            .map(|tournament| {
                                                view! {
                                                    <div>
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
                                    each=move || notifications_context.tournament_finished.get()
                                    key=|id| id.clone()
                                    let:tournament_id
                                >
                                    {move || {
                                        tournaments
                                            .get_value()
                                            .get(&tournament_id)
                                            .map(|tournament| {
                                                view! {
                                                    <div>
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
