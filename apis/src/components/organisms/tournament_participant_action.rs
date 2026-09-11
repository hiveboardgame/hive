use crate::{
    common::{tournament_admission_viewer, tournament_path_matches, TournamentAction},
    components::{
        atoms::login_button::LoginButton,
        organisms::{arena_controls::ArenaAction, tournament_withdrawal::TournamentWithdrawal},
    },
    hooks::arena_clock::use_ticking_now,
    providers::{
        ApiRequestsProvider,
        ArenaStateStoreFields,
        AuthContext,
        AuthIdentity,
        EliminationStateStoreFields,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
        TournamentFormatStore,
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
use shared_types::{tournament::Format, Clock, TournamentStatus};

#[component]
pub fn TournamentParticipantAction(
    common: Store<TournamentCommon>,
    format: TournamentFormatStore,
    #[prop(optional, into)] next_action: Signal<Option<(String, String)>>,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let now = use_ticking_now();
    let tournament_id = StoredValue::new(common.lifecycle().get_untracked().tournament_id);
    let pathname = use_location().pathname;
    let route_active = move || {
        let expected = tournament_id.get_value();
        tournament_path_matches(&pathname.get(), &expected)
            && common.lifecycle().get().tournament_id == expected
    };
    let user_id = Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let relationship = Signal::derive(move || {
        let memberships = common.memberships().get();
        let id = user_id.get();
        ViewerRelationship {
            joined: id.is_some_and(|id| memberships.players.contains_key(&id)),
            invited: id.is_some_and(|id| memberships.invitees.iter().any(|user| user.uid == id)),
            organizing: id
                .is_some_and(|id| memberships.organizers.iter().any(|user| user.uid == id)),
        }
    });
    let entry_open = Signal::derive(move || {
        let lifecycle = common.lifecycle().get();
        lifecycle.status == TournamentStatus::NotStarted
            && !lifecycle
                .start_setup
                .is_some_and(|setup| setup.active_at(now.get()))
            && lifecycle
                .starts_at
                .is_none_or(|starts_at| now.get() < starts_at)
    });
    let admission_auth = auth.clone();
    let admission = Signal::derive(move || {
        let lifecycle = common.lifecycle().get();
        let clock = match format {
            TournamentFormatStore::Arena(state) => {
                Some(Clock::Realtime(state.configuration().get().game_clock))
            }
            TournamentFormatStore::RoundRobin(state) => Some(state.configuration().get().clock),
            TournamentFormatStore::Swiss(state) => Some(state.configuration().get().clock),
            TournamentFormatStore::Elimination(state) => state
                .configuration()
                .get()
                .default_plan
                .phases
                .first()
                .map(|phase| phase.clock),
        };
        TournamentAdmission {
            entry_open: entry_open.get(),
            full: format.format() != Format::Arena
                && lifecycle
                    .seats
                    .and_then(|seats| usize::try_from(seats).ok())
                    .is_none_or(|capacity| common.memberships().get().players.len() >= capacity),
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

    view! {
        <Show when=route_active>
            {move || {
                let status = common.lifecycle().get().status;
                match (status, format) {
                    (TournamentStatus::NotStarted, _) if entry_open.get() => {
                        if matches!(admission.get(), TournamentAccessState::LoginRequired) {
                            return view! {
                                <LoginButton class="ui-button ui-button-primary ui-button-sm no-link-style" />
                            }
                                .into_any();
                        }
                        if relationship.get().joined {
                            return view! {
                                <div class="flex flex-wrap gap-2 items-center">
                                    // TODO: i18n once copy is approved.
                                    <span class="text-sm font-medium">
                                        "Joined · waiting to start"
                                    </span>
                                    <button
                                        type="button"
                                        class="ui-button ui-button-ghost ui-button-sm"
                                        on:click=move |_| {
                                            send.run(TournamentAction::Leave(tournament_id.get_value()))
                                        }
                                    >
                                        // TODO: i18n once copy is approved.
                                        "Leave"
                                    </button>
                                </div>
                            }
                                .into_any();
                        }
                        view! {
                            <div class="flex flex-col gap-1 items-start">
                                <div class="flex flex-wrap gap-2 items-center">
                                    <button
                                        type="button"
                                        class="ui-button ui-button-primary ui-button-sm"
                                        prop:disabled=move || !admission.get().can_enter()
                                        on:click=move |_| {
                                            if !admission.get_untracked().can_enter() {
                                                return;
                                            }
                                            send.run(
                                                if relationship.get_untracked().invited {
                                                    TournamentAction::InvitationAccept(
                                                        tournament_id.get_value(),
                                                    )
                                                } else {
                                                    TournamentAction::Join(tournament_id.get_value())
                                                },
                                            );
                                        }
                                    >
                                        // TODO: i18n once copy is approved.
                                        {move || {
                                            if relationship.get().invited {
                                                "Accept invitation"
                                            } else {
                                                "Join"
                                            }
                                        }}
                                    </button>
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
                    (TournamentStatus::InProgress, TournamentFormatStore::Arena(arena)) => {
                        view! { <ArenaAction common arena /> }.into_any()
                    }
                    (TournamentStatus::InProgress, _) => {
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
                                                    common.memberships().get().withdrawn.contains(&id)
                                                })
                                            {
                                                "Withdrawn"
                                            } else if relationship.get().joined {
                                                "Playing"
                                            } else {
                                                "Watching"
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
                    (TournamentStatus::NotStarted, _) => {
                        view! {
                            // TODO: i18n once copy is approved.
                            <span class="text-sm text-gray-600 dark:text-gray-300">
                                "Entry closed · waiting to start"
                            </span>
                        }
                            .into_any()
                    }
                    (TournamentStatus::Finished, _) => ().into_any(),
                }
            }}
        </Show>
    }
}
