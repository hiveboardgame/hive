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
    i18n::*,
    providers::{
        challenges::ChallengeStateSignal,
        chat::Chat,
        NotificationContext,
        SchedulesContext,
    },
};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::GameId;

#[component]
pub fn NotificationDropdown(current_game_id: Signal<Option<GameId>>) -> impl IntoView {
    let i18n = use_i18n();
    let hamburger_show = RwSignal::new(false);
    let challenges = expect_context::<ChallengeStateSignal>();
    let chat = expect_context::<Chat>();
    let notifications_context = expect_context::<NotificationContext>();
    let schedules_context = expect_context::<SchedulesContext>();
    let latest_chat_unread_message_id = Memo::new(move |_| {
        let current_game_id = current_game_id.get();
        chat.latest_unread_message_id_excluding_game(current_game_id.as_ref())
    });
    let dismissed_unread_message_id = RwSignal::new(0_i64);
    let has_chat_notification =
        move || latest_chat_unread_message_id.get() > dismissed_unread_message_id.get();
    let has_notifications = move || !notifications_context.is_empty() || has_chat_notification();

    let icon_style = move || {
        if has_notifications() {
            "inline-flex text-ladybug-red"
        } else {
            "inline-flex"
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
            content=view! {
                <span class=icon_style>
                    <Icon icon=icondata_io::IoNotifications attr:class="size-4" />
                </span>
            }
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
                    <Show when=has_chat_notification>
                        <div class="ui-notification-item">
                            <div class="relative flex-1 min-w-0">
                                <div class="ui-notification-label">
                                    {t!(i18n, messages.page.title)}
                                </div>
                                <div class="ui-notification-title">
                                    {t!(i18n, messages.chat.new_chat_message)}
                                </div>
                                <a
                                    class="absolute top-0 left-0 z-10 size-full"
                                    href="/message"
                                    aria-label=move || t_string!(i18n, header.user_menu.messages)
                                    on:click=move |_| hamburger_show.set(false)
                                ></a>
                            </div>
                            <button
                                type="button"
                                title=move || t_string!(i18n, messages.chat.dismiss)
                                aria-label=move || t_string!(i18n, messages.chat.dismiss)
                                on:click=move |event| {
                                    event.prevent_default();
                                    event.stop_propagation();
                                    dismissed_unread_message_id
                                        .set(latest_chat_unread_message_id.get_untracked());
                                }
                                class="z-20 ui-button ui-button-ghost ui-button-icon"
                            >
                                <Icon icon=icondata_io::IoCloseSharp attr:class="size-4" />
                            </button>
                        </div>
                    </Show>

                    <For each=notifications_context.challenges key=|c| c.clone() let:challenge_id>
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
