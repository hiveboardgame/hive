use crate::{
    common::{tournament_admission_viewer, TournamentAction},
    components::atoms::login_button::LoginButton,
    hooks::arena_clock::use_ticking_now,
    i18n::*,
    providers::{
        ApiRequestsProvider,
        ArenaState,
        ArenaStateStoreFields,
        AuthContext,
        AuthIdentity,
        TournamentCommon,
        TournamentCommonStoreFields,
    },
    responses::{AdmissionRestrictions, TournamentAdmission, ViewerRelationship},
};
use chrono::{DateTime, Duration, Utc};
use leptos::prelude::*;
use reactive_stores::{ArcField, Store};
use shared_types::{
    tournament_view::{ArenaGameResponse, ArenaPlayerStatsResponse},
    Clock,
    TournamentStatus,
};

fn arena_entry_open(
    lifecycle_accepts_join: bool,
    now: DateTime<Utc>,
    starts_at: DateTime<Utc>,
    pairing_closes_at: DateTime<Utc>,
) -> bool {
    lifecycle_accepts_join && now >= starts_at && now < pairing_closes_at
}

fn arena_intent_controls(
    paused: bool,
    now: DateTime<Utc>,
    pairing_closes_at: DateTime<Utc>,
) -> (bool, bool) {
    if now >= pairing_closes_at {
        return (!paused, false);
    }
    (!paused, paused)
}

#[component]
pub fn ArenaAction(common: Store<TournamentCommon>, arena: Store<ArenaState>) -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;
    let now = use_ticking_now();

    let user_id =
        Signal::derive(move || auth_context.identity.get().and_then(AuthIdentity::user_id));
    let arena_times = Signal::derive(move || {
        let lifecycle = common.lifecycle().get();
        lifecycle
            .starts_at
            .zip(Some(arena.configuration().get().duration_seconds.get()))
            .map(|(starts_at, duration)| {
                let ends_at = starts_at + Duration::seconds(duration as i64);
                let pairing_closes_at = ends_at - Duration::seconds(60);
                (starts_at, pairing_closes_at, ends_at)
            })
    });
    let is_running_arena = Signal::derive(move || {
        let lifecycle = common.lifecycle().get();
        match lifecycle.status {
            TournamentStatus::InProgress => true,
            TournamentStatus::NotStarted => {
                arena_times.get().is_some_and(|(starts_at, _, ends_at)| {
                    now.get() >= starts_at && now.get() < ends_at
                })
            }
            TournamentStatus::Finished => false,
        }
    });
    let is_entrant = Signal::derive(move || match user_id.get() {
        Some(id) => common.memberships().read().players.contains_key(&id),
        None => false,
    });
    let viewer_games = Signal::derive(move || {
        let id = user_id.get()?;
        let game_ids = arena
            .games()
            .with(|games| games.keys().cloned().collect::<Vec<_>>());
        Some(
            game_ids
                .into_iter()
                .map(|game_id| -> ArcField<ArenaGameResponse> {
                    arena.games().at_key(game_id).into()
                })
                .filter(|game| game.get_untracked().game.participants.contains(&id))
                .collect::<Vec<_>>(),
        )
    });
    let own_active_game = Signal::derive(move || {
        viewer_games.get()?.into_iter().find_map(|game| {
            let game = game.try_get()?;
            (!game.game.finished).then(|| game.game.game_id.clone())
        })
    });
    let entry_window_open = Signal::derive(move || {
        arena_times.get().is_some_and(|(start, close, _)| {
            arena_entry_open(is_running_arena.get(), now.get(), start, close)
        })
    });
    let admission_auth = auth_context.clone();
    let admission = Signal::derive(move || {
        let lifecycle = common.lifecycle().get();
        let memberships = common.memberships().get();
        let clock = Some(Clock::Realtime(arena.configuration().get().game_clock));
        let id = user_id.get();
        TournamentAdmission {
            entry_open: entry_window_open.get(),
            full: false,
            restrictions: AdmissionRestrictions {
                invite_only: lifecycle.invite_only,
                band_lower: lifecycle.band_lower,
                band_upper: lifecycle.band_upper,
            },
            relationship: ViewerRelationship {
                joined: is_entrant.get(),
                invited: id
                    .is_some_and(|id| memberships.invitees.iter().any(|user| user.uid == id)),
                organizing: id
                    .is_some_and(|id| memberships.organizers.iter().any(|user| user.uid == id)),
            },
            clock,
            bot_admission: common.bot_admission().get(),
        }
        .decision(tournament_admission_viewer(&admission_auth, clock))
    });
    let can_join = Signal::derive(move || admission.get().can_enter());
    let can_manage = Signal::derive(move || {
        let Some(id) = user_id.get() else {
            return false;
        };
        common.lifecycle().get().status == TournamentStatus::InProgress
            && common.memberships().read().players.contains_key(&id)
    });
    let viewer_stats = Signal::derive(move || {
        let id = user_id.get()?;
        arena
            .player_stats()
            .with(|stats| stats.contains_key(&id))
            .then(|| -> ArcField<ArenaPlayerStatsResponse> {
                arena.player_stats().at_key(id).into()
            })
    });
    let own_is_paused = Signal::derive(move || {
        user_id.get()?;
        Some(
            viewer_stats
                .get()
                .is_some_and(|stats| stats.try_get().is_some_and(|stats| stats.paused)),
        )
    });
    let can_pause = Signal::derive(move || {
        let Some((_, pairing_closes_at, _)) = arena_times.get() else {
            return false;
        };
        can_manage.get()
            && arena_intent_controls(
                own_is_paused.get().unwrap_or_default(),
                now.get(),
                pairing_closes_at,
            )
            .0
    });
    let can_resume = Signal::derive(move || {
        let Some((_, pairing_closes_at, _)) = arena_times.get() else {
            return false;
        };
        can_manage.get()
            && arena_intent_controls(
                own_is_paused.get().unwrap_or_default(),
                now.get(),
                pairing_closes_at,
            )
            .1
    });
    let send = move |action: TournamentAction| {
        move |_| {
            api.get().tournament(action.clone());
        }
    };
    let tournament_id = StoredValue::new(
        common
            .lifecycle()
            .with_untracked(|lifecycle| lifecycle.tournament_id.clone()),
    );

    view! {
        <Show when=is_running_arena>
            <div class="flex flex-col gap-1 items-end min-w-0">
                <Show when=is_entrant>
                    // TODO: i18n once copy is approved.
                    <p class="text-xs text-gray-600 dark:text-gray-300">
                        {move || {
                            if own_active_game.get().is_some() {
                                "Playing"
                            } else if own_is_paused.get().unwrap_or_default() {
                                "Paused"
                            } else if !entry_window_open.get() {
                                "Pairings closed"
                            } else {
                                "Waiting for an opponent"
                            }
                        }}
                    </p>
                </Show>
                <Show when=move || !is_entrant.get() && !admission.get().can_enter()>
                    <p class="text-xs text-gray-600 dark:text-gray-300">
                        {move || admission.get().label()}
                    </p>
                </Show>
                <div class="flex flex-wrap gap-2 justify-end">
                    <Show when=move || own_active_game.get().is_some()>
                        <a
                            class="whitespace-nowrap ui-button ui-button-primary ui-button-sm"
                            href=move || {
                                own_active_game
                                    .get()
                                    .map(|game_id| format!("/game/{}", game_id.0))
                                    .unwrap_or_default()
                            }
                        >
                            {t!(i18n, tournaments.view.arena.current_game)}
                        </a>
                    </Show>
                    <Show when=move || {
                        entry_window_open.get() && !is_entrant.get()
                    }>
                        {move || match auth_context.identity.get() {
                            None => {
                                view! {
                                    <button
                                        class="ui-button ui-button-primary ui-button-sm"
                                        disabled
                                    >
                                        {t!(i18n, tournaments.arena.join)}
                                    </button>
                                }
                                    .into_any()
                            }
                            Some(AuthIdentity::Anonymous) => {
                                view! {
                                    <LoginButton class="ui-button ui-button-primary ui-button-sm no-link-style" />
                                }
                                    .into_any()
                            }
                            Some(AuthIdentity::User(_)) => {
                                view! {
                                    <button
                                        class="ui-button ui-button-primary ui-button-sm"
                                        prop:disabled=move || !can_join.get()
                                        on:click=send(
                                            TournamentAction::ArenaJoin(tournament_id.get_value()),
                                        )
                                    >
                                        {t!(i18n, tournaments.arena.join)}
                                    </button>
                                }
                                    .into_any()
                            }
                        }}
                    </Show>
                    <Show when=can_pause>
                        // TODO: i18n once copy is approved.
                        <button
                            class="ui-button ui-button-secondary ui-button-sm"
                            title="Pause stops new pairings. Your current game continues."
                            on:click=send(TournamentAction::ArenaPause(tournament_id.get_value()))
                        >
                            {t!(i18n, tournaments.arena.pause)}
                        </button>
                    </Show>
                    <Show when=can_resume>
                        <button
                            class="ui-button ui-button-secondary ui-button-sm"
                            title=move || {
                                t_string!(i18n, tournaments.arena.resume_title).to_string()
                            }
                            on:click=send(TournamentAction::ArenaResume(tournament_id.get_value()))
                        >
                            {t!(i18n, tournaments.arena.resume)}
                        </button>
                    </Show>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delayed_worker_join_uses_scheduled_origin_and_excludes_exact_cutoff() {
        let starts_at = DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp");
        let pairing_closes_at = starts_at + Duration::hours(1) - Duration::seconds(60);

        assert!(!arena_entry_open(
            true,
            starts_at - Duration::nanoseconds(1),
            starts_at,
            pairing_closes_at,
        ));
        assert!(arena_entry_open(
            true,
            starts_at,
            starts_at,
            pairing_closes_at,
        ));
        assert!(!arena_entry_open(
            true,
            pairing_closes_at,
            starts_at,
            pairing_closes_at,
        ));
    }

    #[test]
    fn pause_remains_available_but_resume_closes_at_the_pairing_cutoff() {
        let cutoff = DateTime::from_timestamp(1_700_000_000, 0).expect("valid timestamp");
        let before = cutoff - Duration::nanoseconds(1);

        assert_eq!(arena_intent_controls(false, before, cutoff), (true, false),);
        assert_eq!(arena_intent_controls(true, before, cutoff), (false, true),);
        assert_eq!(arena_intent_controls(false, cutoff, cutoff), (true, false),);
        assert_eq!(arena_intent_controls(true, cutoff, cutoff), (false, false),);
    }
}
