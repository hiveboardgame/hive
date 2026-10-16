use crate::{
    common::{tournament_admission_viewer, tournament_path_matches, TournamentAction},
    components::{
        atoms::login_button::LoginButton,
        organisms::{
            arena_controls::ArenaAction,
            tournament_entry::TournamentJoinButton,
            tournament_withdrawal::TournamentWithdrawal,
        },
    },
    hooks::arena_clock::use_ticking_now,
    providers::{
        ApiRequestsProvider,
        ArenaState,
        ArenaStateStoreFields,
        AuthContext,
        AuthIdentity,
        EliminationStateStoreFields,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
        TournamentFormatState,
    },
    responses::{
        AdmissionRestrictions,
        TournamentAccessState,
        TournamentAdmission,
        ViewerRelationship,
    },
};
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use reactive_stores::Store;
use shared_types::{Clock, TournamentStatus};

#[derive(Clone, Copy, PartialEq)]
enum ParticipantAction {
    LoginRequired,
    Joined,
    Join,
    Arena(Store<ArenaState>),
    Playing,
    EntryClosed,
    Hidden,
}

#[component]
pub fn TournamentParticipantAction(
    common: Store<TournamentCommon>,
    format: TournamentFormatState,
    #[prop(optional, into)] next_action: Signal<Option<(String, String)>>,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let now = use_ticking_now();
    let tournament_id = StoredValue::new(
        common
            .lifecycle()
            .with_untracked(|lifecycle| lifecycle.tournament_id.clone()),
    );
    let pathname = use_location().pathname;
    let route_active = move || {
        let expected = tournament_id.get_value();
        tournament_path_matches(&pathname.get(), &expected)
            && common
                .lifecycle()
                .with(|lifecycle| lifecycle.tournament_id == expected)
    };
    let user_id = Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let relationship = Signal::derive(move || {
        let memberships = common.memberships().read();
        let id = user_id.get();
        ViewerRelationship {
            joined: id.is_some_and(|id| memberships.players.contains_key(&id)),
            invited: id.is_some_and(|id| memberships.invitees.iter().any(|user| user.uid == id)),
            organizing: id
                .is_some_and(|id| memberships.organizers.iter().any(|user| user.uid == id)),
        }
    });
    let entry_open = Memo::new(move |_| {
        let lifecycle = common.lifecycle().read();
        lifecycle.status == TournamentStatus::NotStarted
            && !lifecycle
                .start_setup
                .as_ref()
                .is_some_and(|setup| setup.active_at(now.get()))
            && lifecycle
                .starts_at
                .is_none_or(|starts_at| now.get() < starts_at)
    });
    let admission_auth = auth.clone();
    let admission = Signal::derive(move || {
        let lifecycle = common.lifecycle().read();
        let clock = match format {
            TournamentFormatState::Arena(state) => Some(Clock::Realtime(
                state
                    .configuration()
                    .with(|configuration| configuration.game_clock),
            )),
            TournamentFormatState::RoundRobin(state) => Some(
                state
                    .configuration()
                    .with(|configuration| configuration.clock),
            ),
            TournamentFormatState::Swiss(state) => Some(
                state
                    .configuration()
                    .with(|configuration| configuration.clock),
            ),
            TournamentFormatState::Elimination(state) => {
                state.configuration().with(|configuration| {
                    configuration
                        .default_plan
                        .phases
                        .first()
                        .map(|phase| phase.clock)
                })
            }
        };
        TournamentAdmission {
            entry_open: entry_open.get(),
            full: !matches!(format, TournamentFormatState::Arena(_))
                && lifecycle
                    .seats
                    .and_then(|seats| usize::try_from(seats).ok())
                    .is_some_and(|capacity| {
                        common
                            .memberships()
                            .with(|memberships| memberships.players.len() >= capacity)
                    }),
            restrictions: AdmissionRestrictions {
                invite_only: lifecycle.invite_only,
                band_lower: lifecycle.band_lower,
                band_upper: lifecycle.band_upper,
            },
            relationship: relationship.get(),
            clock,
            bot_admission: common.bot_admission().get(),
        }
        .decision(tournament_admission_viewer(&admission_auth, clock))
    });
    let send = Callback::new(move |action: TournamentAction| {
        if route_active() {
            api.get().tournament(action);
        }
    });

    // Read branch conditions only here: unchanged actions must preserve dialog owners.
    let action_state =
        Memo::new(
            move |_| match common.lifecycle().with(|lifecycle| lifecycle.status) {
                TournamentStatus::NotStarted if entry_open.get() => {
                    if matches!(admission.get(), TournamentAccessState::LoginRequired) {
                        ParticipantAction::LoginRequired
                    } else if relationship.get().joined {
                        ParticipantAction::Joined
                    } else {
                        ParticipantAction::Join
                    }
                }
                TournamentStatus::NotStarted => ParticipantAction::EntryClosed,
                TournamentStatus::InProgress => match format {
                    TournamentFormatState::Arena(arena) => ParticipantAction::Arena(arena),
                    _ if relationship.get().joined => ParticipantAction::Playing,
                    _ => ParticipantAction::Hidden,
                },
                TournamentStatus::Finished => ParticipantAction::Hidden,
            },
        );

    view! {
        <Show when=route_active>
            {move || {
                match action_state.get() {
                    ParticipantAction::LoginRequired => {
                        view! {
                            <LoginButton class="ui-button ui-button-primary ui-button-sm no-link-style" />
                        }
                            .into_any()
                    }
                    ParticipantAction::Joined => {
                        view! {
                            <div class="flex flex-wrap gap-2 items-center">
                                // TODO: i18n once copy is approved.
                                <span class="text-sm font-medium">
                                    "Joined · waiting to start"
                                </span>
                                <button
                                    type="button"
                                    class="ui-button ui-button-danger ui-button-sm"
                                    on:click=move |_| {
                                        send.run(TournamentAction::Leave(tournament_id.get_value()))
                                    }
                                >
                                    // TODO: i18n once copy is approved.
                                    "Leave"
                                </button>
                            </div>
                        }
                            .into_any()
                    }
                    ParticipantAction::Join => {
                        view! {
                            <div class="flex flex-col gap-1 items-start">
                                <div class="flex flex-wrap gap-2 items-center">
                                    <TournamentJoinButton
                                        common
                                        format
                                        can_join=Signal::derive(move || admission.get().can_enter())
                                        // TODO: i18n once copy is approved.
                                        label=Signal::derive(move || {
                                            if relationship.get().invited {
                                                "Accept invitation"
                                            } else {
                                                "Join"
                                            }
                                                .to_string()
                                        })
                                        on_join=Callback::new(move |()| {
                                            send.run(
                                                if relationship.get_untracked().invited {
                                                    TournamentAction::InvitationAccept(
                                                        tournament_id.get_value(),
                                                    )
                                                } else {
                                                    TournamentAction::Join(tournament_id.get_value())
                                                },
                                            );
                                        })
                                    />
                                    <Show when=move || relationship.get().invited>
                                        <button
                                            type="button"
                                            class="ui-button ui-button-ghost ui-button-sm"
                                            on:click=move |_| {
                                                send.run(
                                                    TournamentAction::InvitationDecline(
                                                        tournament_id.get_value(),
                                                    ),
                                                )
                                            }
                                        >
                                            // TODO: i18n once copy is approved.
                                            "Decline"
                                        </button>
                                    </Show>
                                </div>
                                <Show when=move || !admission.get().can_enter()>
                                    <span
                                        class="text-xs text-gray-600 dark:text-gray-300"
                                        role="status"
                                    >
                                        {move || admission.get().label()}
                                    </span>
                                </Show>
                            </div>
                        }
                            .into_any()
                    }
                    ParticipantAction::Arena(arena) => {
                        view! { <ArenaAction common arena /> }.into_any()
                    }
                    ParticipantAction::Playing => {
                        view! {
                            <div class="flex flex-wrap gap-y-2 gap-x-4 items-center">
                                {move || {
                                    next_action
                                        .get()
                                        .map(|(label, href)| {
                                            view! {
                                                <a
                                                    class="ui-button ui-button-primary ui-button-sm no-link-style"
                                                    href=href
                                                >
                                                    {label}
                                                </a>
                                            }
                                        })
                                }} <div class="flex gap-2 items-center">
                                    <span
                                        class="text-sm text-gray-600 dark:text-gray-300"
                                        role="status"
                                    >
                                        // TODO: i18n once copy is approved.
                                        {move || {
                                            if user_id
                                                .get()
                                                .is_some_and(|id| {
                                                    common
                                                        .memberships()
                                                        .with(|memberships| memberships.withdrawn.contains(&id))
                                                })
                                            {
                                                "Withdrawn"
                                            } else if let TournamentFormatState::Elimination(state) = format {
                                                state
                                                    .player_results()
                                                    .with(|results| {
                                                        match results
                                                            .iter()
                                                            .find(|result| Some(result.player) == user_id.get())
                                                        {
                                                            Some(result) if result.placement == Some(1) => "Winner",
                                                            Some(result) if !result.in_contention => "Eliminated",
                                                            _ => "Playing",
                                                        }
                                                    })
                                            } else {
                                                "Playing"
                                            }
                                        }}
                                    </span>
                                    <TournamentWithdrawal
                                        common
                                        format
                                        organizer=Signal::derive(|| false)
                                    />
                                </div>
                            </div>
                        }
                            .into_any()
                    }
                    ParticipantAction::EntryClosed => {
                        view! {
                            // TODO: i18n once copy is approved.
                            <span class="text-sm text-gray-600 dark:text-gray-300">
                                "Entry closed · waiting to start"
                            </span>
                        }
                            .into_any()
                    }
                    ParticipantAction::Hidden => ().into_any(),
                }
            }}
        </Show>
    }
}
