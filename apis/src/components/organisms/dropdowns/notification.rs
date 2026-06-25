use crate::{
    components::{
        atoms::challenge_details::ChallengeDetails,
        molecules::{
            hamburger::Hamburger,
            schedule_notification::{AcceptanceNotification, ProposalNotification},
            tournament_invitation_notification::TournamentInvitationNotification,
            tournament_status_notification::TournamentStatusNotification,
        },
    },
    functions::tournaments::get_abstracts_by_ids,
    providers::{challenges::ChallengeStateSignal, NotificationContext, SchedulesContext},
};
use leptos::prelude::*;
use leptos_icons::*;

#[component]
pub fn NotificationDropdown() -> impl IntoView {
    let hamburger_show = RwSignal::new(false);
    let challenges = expect_context::<ChallengeStateSignal>();
    let notifications_context = expect_context::<NotificationContext>();
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

    let tournaments_resource = LocalResource::new(move || {
        let tournament_ids = notifications_context.sorted_tournament_notification_ids();
        async move {
            if tournament_ids.is_empty() {
                Ok(Vec::new())
            } else {
                get_abstracts_by_ids(tournament_ids.into_iter().collect()).await
            }
        }
    });

    view! {
        <Hamburger
            hamburger_show=hamburger_show
            button_style="ui-header-icon-button"
            extend_tw_classes="h-full"
            dropdown_style="ui-dropdown-menu ui-dropdown-menu-right ui-header-dropdown-menu ui-notification-dropdown-menu"
            content=view! { <Icon icon=icondata_io::IoNotifications attr:class=icon_style /> }
            id="Notifications"
            aria_label="Open notifications"
        >
            <Show
                when=has_notifications
                fallback=|| {
                    view! { <div class="ui-notification-empty">"No notifications right now"</div> }
                }
            >
                <div class="ui-notification-list">
                    <For
                        each=move || notifications_context.challenges.get()
                        key=|c| c.clone()
                        let:challenge_id
                    >
                        {
                            let challenge = Signal::derive(move || {
                                challenges
                                    .signal
                                    .with(|state| { state.challenges.get(&challenge_id).cloned() })
                            });

                            view! {
                                {move || {
                                    challenge
                                        .get()
                                        .map(|challenge| {
                                            view! {
                                                <ChallengeDetails
                                                    challenge=challenge
                                                    label="Direct Challenge"
                                                    class="ui-notification-item"
                                                />
                                            }
                                        })
                                }}
                            }
                        }
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
                        <ProposalNotification
                            schedule_id=schedule.id
                            proposer_username=schedule.proposer_username
                            tournament_id=schedule.tournament_id
                            start_time=schedule.start_t
                        />
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
                        <AcceptanceNotification
                            tournament_name=schedule.tournament_name
                            schedule_id=schedule.id
                            accepter_username=schedule.opponent_username
                            tournament_id=schedule.tournament_id
                            start_time=schedule.start_t
                        />
                    </For>

                    <Show when=has_tournament_notifications>
                        <Transition fallback=|| {
                            "Loading tournaments..."
                        }>
                            {move || {
                                tournaments_resource
                                    .get()
                                    .map(|tournaments| {
                                        let invitation_ids = notifications_context
                                            .tournament_invitations();
                                        let started_ids = notifications_context
                                            .tournament_started
                                            .get();
                                        let finished_ids = notifications_context
                                            .tournament_finished
                                            .get();
                                        let (invitations, started, finished) = tournaments
                                            .unwrap_or_default()
                                            .into_iter()
                                            .fold(
                                                (Vec::new(), Vec::new(), Vec::new()),
                                                |mut notifications, tournament| {
                                                    let tournament_id = tournament.tournament_id.clone();
                                                    if invitation_ids.contains(&tournament_id) {
                                                        notifications.0.push(tournament.clone());
                                                    }
                                                    if started_ids.contains(&tournament_id) {
                                                        notifications
                                                            .1
                                                            .push((tournament_id.clone(), tournament.name.clone()));
                                                    }
                                                    if finished_ids.contains(&tournament_id) {
                                                        notifications.2.push((tournament_id, tournament.name));
                                                    }
                                                    notifications
                                                },
                                            );

                                        view! {
                                            <For
                                                each=move || invitations.clone()
                                                key=|tournament| {
                                                    (tournament.tournament_id.clone(), tournament.players)
                                                }
                                                let:tournament
                                            >
                                                <TournamentInvitationNotification tournament=tournament />
                                            </For>

                                            <For
                                                each=move || started.clone()
                                                key=|status| status.0.clone()
                                                let:status
                                            >
                                                <TournamentStatusNotification
                                                    tournament_id=status.0
                                                    tournament_name=status.1
                                                    finished=false
                                                />
                                            </For>

                                            <For
                                                each=move || finished.clone()
                                                key=|status| status.0.clone()
                                                let:status
                                            >
                                                <TournamentStatusNotification
                                                    tournament_id=status.0
                                                    tournament_name=status.1
                                                    finished=true
                                                />
                                            </For>
                                        }
                                    })
                            }}
                        </Transition>
                    </Show>
                </div>
            </Show>
        </Hamburger>
    }
}
