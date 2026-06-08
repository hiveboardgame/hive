use crate::{
    components::molecules::{
        challenge_row::ChallengeRow,
        hamburger::Hamburger,
        schedule_notification::{AcceptanceNotification, ProposalNotification},
        tournament_invitation_notification::TournamentInvitationNotification,
        tournament_status_notification::TournamentStatusNotification,
    },
    functions::tournaments::get_abstracts_by_ids,
    providers::{challenges::ChallengeStateSignal, NotificationContext, SchedulesContext},
};
use leptos::prelude::*;
use leptos_icons::*;
use std::collections::HashMap;

#[component]
pub fn NotificationDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let notifications_context = expect_context::<NotificationContext>();
    let challenges = expect_context::<ChallengeStateSignal>();
    let schedules_context = expect_context::<SchedulesContext>();
    let has_notifications = move || !notifications_context.is_empty();

    let icon_style = move || {
        if has_notifications() {
            "size-4 fill-ladybug-red"
        } else {
            "size-4"
        }
    };

    let has_tournament_notifications = move || notifications_context.has_tournament_notifications();

    let tournaments_resource = Resource::new(
        move || notifications_context.sorted_tournament_notification_ids(),
        move |tournament_ids| get_abstracts_by_ids(tournament_ids.into_iter().collect()),
    );

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="h-full p-2 transform transition-transform duration-300 active:scale-95 whitespace-nowrap block"
            dropdown_style="mr-1 items-center xs:mt-0 mt-1 flex flex-col items-stretch absolute w-max bg-even-light dark:bg-gray-950 border border-gray-300 rounded-md p-2 right-0 z-50"
            content=view! { <Icon icon=icondata_io::IoNotifications attr:class=icon_style /> }
            id="Notifications"
        >
            <Show when=has_notifications fallback=|| "No notifications right now">
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
                                    attr:class="p-2 text-sm rounded dark:bg-header-twilight bg-odd-light"
                                />
                            </tbody>
                        </table>
                    </div>
                </For>

                <For
                    each=move || {
                        schedules_context
                            .own
                            .with(|schedules| {
                                notifications_context
                                    .schedule_proposals()
                                    .into_iter()
                                    .filter_map(|schedule_id| {
                                        schedules
                                            .values()
                                            .find_map(|game_schedules| {
                                                game_schedules.get(&schedule_id).cloned()
                                            })
                                    })
                                    .collect::<Vec<_>>()
                            })
                    }
                    key=|schedule| schedule.id
                    let:schedule
                >
                    <div>
                        <ProposalNotification
                            schedule_id=schedule.id
                            proposer_username=schedule.proposer_username
                            tournament_id=schedule.tournament_id
                            start_time=schedule.start_t
                        />
                    </div>
                </For>

                <For
                    each=move || {
                        schedules_context
                            .own
                            .with(|schedules| {
                                notifications_context
                                    .schedule_acceptances()
                                    .into_iter()
                                    .filter_map(|schedule_id| {
                                        schedules
                                            .values()
                                            .find_map(|game_schedules| {
                                                game_schedules.get(&schedule_id).cloned()
                                            })
                                    })
                                    .collect::<Vec<_>>()
                            })
                    }
                    key=|schedule| schedule.id
                    let:schedule
                >
                    <div>
                        <AcceptanceNotification
                            tournament_name=schedule.tournament_name
                            schedule_id=schedule.id
                            accepter_username=schedule.opponent_username
                            tournament_id=schedule.tournament_id
                            start_time=schedule.start_t
                        />
                    </div>
                </For>

                <Show when=has_tournament_notifications>
                    <Transition fallback=|| {
                        "Loading tournaments..."
                    }>
                        {move || {
                            tournaments_resource
                                .get()
                                .map(|tournaments| {
                                    let tournaments = tournaments
                                        .unwrap_or_default()
                                        .into_iter()
                                        .map(|tournament| {
                                            (tournament.tournament_id.clone(), tournament)
                                        })
                                        .collect::<HashMap<_, _>>();
                                    let tournaments = StoredValue::new(tournaments);

                                    view! {
                                        <For
                                            each=move || {
                                                tournaments
                                                    .with_value(|tournaments| {
                                                        notifications_context
                                                            .tournament_invitations()
                                                            .into_iter()
                                                            .filter_map(|id| tournaments.get(&id).cloned())
                                                            .collect::<Vec<_>>()
                                                    })
                                            }
                                            key=|tournament| {
                                                (tournament.tournament_id.clone(), tournament.players)
                                            }
                                            let:tournament
                                        >
                                            <div>
                                                <TournamentInvitationNotification tournament=tournament />
                                            </div>
                                        </For>

                                        <For
                                            each=move || {
                                                tournaments
                                                    .with_value(|tournaments| {
                                                        notifications_context
                                                            .tournament_started
                                                            .get()
                                                            .into_iter()
                                                            .filter_map(|id| {
                                                                tournaments
                                                                    .get(&id)
                                                                    .map(|tournament| { (id, tournament.name.clone()) })
                                                            })
                                                            .collect::<Vec<_>>()
                                                    })
                                            }
                                            key=|status| status.0.clone()
                                            let:status
                                        >
                                            <div>
                                                <TournamentStatusNotification
                                                    tournament_id=status.0
                                                    tournament_name=status.1
                                                    finished=false
                                                />
                                            </div>
                                        </For>

                                        <For
                                            each=move || {
                                                tournaments
                                                    .with_value(|tournaments| {
                                                        notifications_context
                                                            .tournament_finished
                                                            .get()
                                                            .into_iter()
                                                            .filter_map(|id| {
                                                                tournaments
                                                                    .get(&id)
                                                                    .map(|tournament| { (id, tournament.name.clone()) })
                                                            })
                                                            .collect::<Vec<_>>()
                                                    })
                                            }
                                            key=|status| status.0.clone()
                                            let:status
                                        >
                                            <div>
                                                <TournamentStatusNotification
                                                    tournament_id=status.0
                                                    tournament_name=status.1
                                                    finished=true
                                                />
                                            </div>
                                        </For>
                                    }
                                })
                        }}
                    </Transition>
                </Show>
            </Show>
        </Hamburger>
    }
}
