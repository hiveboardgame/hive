use crate::{
    common::{tournament_admission_viewer, TournamentAction},
    components::{
        atoms::rating::icon_for_speed,
        molecules::{panel::Panel, time_row::format_compact_duration},
    },
    functions::tournaments::get_live_arenas,
    hooks::arena_clock::{format_time_left, use_ticking_now},
    i18n::*,
    providers::{ApiRequestsProvider, AuthContext, AuthIdentity, UpdateNotifier},
    responses::{
        AdmissionRestrictions,
        TournamentAbstractResponse,
        TournamentAdmission,
        ViewerRelationship,
    },
};
use chrono::{DateTime, Duration, Utc};
use leptos::prelude::*;
use leptos_icons::*;
use shared_types::{clock::Clock, tournament::FormatConfig, GameSpeed};
use uuid::Uuid;

type ViewerIdentityKey = (bool, Option<Uuid>);

fn viewer_identity_key(identity: Option<AuthIdentity>) -> ViewerIdentityKey {
    (identity.is_some(), identity.and_then(AuthIdentity::user_id))
}

fn resource_identity_matches(current: ViewerIdentityKey, loaded: ViewerIdentityKey) -> bool {
    current == loaded
}

#[component]
pub fn LiveArenas() -> impl IntoView {
    let i18n = use_i18n();
    let update = expect_context::<UpdateNotifier>().tournament_catalog_update;
    let auth = expect_context::<AuthContext>();
    let auth_for_resource = auth.clone();
    let arenas = Resource::new(
        move || {
            (
                update.get(),
                viewer_identity_key(auth_for_resource.identity.get()),
            )
        },
        |(_, identity_key)| async move { (identity_key, get_live_arenas().await) },
    );

    let now = use_ticking_now();

    view! {
        <Transition>
            {move || {
                let current_identity = viewer_identity_key(auth.identity.get());
                arenas
                    .get()
                    .filter(|(loaded_identity, _)| {
                        resource_identity_matches(current_identity, *loaded_identity)
                    })
                    .and_then(|(_, response)| response.ok())
                    .map(|tournaments| {
                        let arenas = tournaments
                            .into_iter()
                            .filter_map(|arena| {
                                let (starts_at, duration) = arena
                                    .scheduled_starts_at()
                                    .zip(arena.arena_duration_seconds())?;
                                let ends_at = starts_at + Duration::seconds(duration as i64);
                                Some((arena, ends_at))
                            })
                            .collect::<Vec<_>>();
                        let latest_end = arenas.iter().map(|(_, ends_at)| *ends_at).max();

                        // Clock ticks only change visibility and card controls; they
                        // must not reread or clone the resource's tournament list.
                        view! {
                            <Show when=move || latest_end.is_some_and(|end| now.get() < end)>
                                <div class="mx-auto w-full">
                                    <Panel
                                        title=move || t_string!(i18n, tournaments.arena.live_title)
                                        clone:arenas
                                    >
                                        <ul class="divide-y divide-gray-200 dark:divide-gray-700">
                                            {arenas
                                                .into_iter()
                                                .map(|(arena, ends_at)| {
                                                    view! {
                                                        <Show when=move || now.get() < ends_at>
                                                            <ArenaCard arena=arena.clone() now ends_at />
                                                        </Show>
                                                    }
                                                })
                                                .collect_view()}
                                        </ul>
                                    </Panel>
                                </div>
                            </Show>
                        }
                    })
            }}
        </Transition>
    }
}

#[component]
fn ArenaCard(
    arena: TournamentAbstractResponse,
    now: Signal<DateTime<Utc>>,
    ends_at: DateTime<Utc>,
) -> impl IntoView {
    let i18n = use_i18n();
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;

    let tournament_id = arena.tournament_id.clone();
    let href = format!("/tournament/{}", tournament_id.0);
    let name = arena.name.clone();
    let players = arena.players;
    let clock = arena.admission_clock();
    let admission_policy = TournamentAdmission {
        entry_open: true,
        full: false,
        restrictions: AdmissionRestrictions {
            invite_only: arena.invite_only,
            band_lower: arena.band_lower,
            band_upper: arena.band_upper,
        },
        relationship: ViewerRelationship {
            joined: arena.joined,
            invited: arena.invited,
            organizing: arena.organizing,
        },
        clock,
        bot_admission: arena.configuration.bot_admission,
    };
    let starts_at = arena.scheduled_starts_at();
    let arena_configuration = match &arena.configuration.format {
        FormatConfig::Arena(configuration) => Some(configuration),
        _ => None,
    };
    let pairing_closes_at = ends_at - Duration::seconds(60);
    let speed = arena_configuration.map_or(GameSpeed::Untimed, |configuration| {
        GameSpeed::from(Clock::Realtime(configuration.game_clock))
    });
    let time_control = arena_configuration.map_or_else(String::new, |configuration| {
        format!(
            "{} + {}",
            format_compact_duration(configuration.game_clock.base_seconds.get()),
            format_compact_duration(configuration.game_clock.increment_seconds)
        )
    });

    let time_left = Signal::derive(move || ends_at - now.get());

    let admission = Signal::derive(move || {
        TournamentAdmission {
            entry_open: starts_at.is_some_and(|start| now.get() >= start)
                && now.get() < pairing_closes_at,
            ..admission_policy
        }
        .decision(tournament_admission_viewer(&auth_context, clock))
    });
    let can_join = Signal::derive(move || admission.get().can_enter());

    let join = move |_| {
        if !can_join.get_untracked() {
            return;
        }
        api.get()
            .tournament(TournamentAction::ArenaJoin(tournament_id.clone()));
    };

    view! {
        <li class="flex gap-3 justify-between items-center py-2 px-1">
            <a
                href=href.clone()
                class="flex flex-col min-w-0 rounded transition-opacity hover:opacity-80 grow"
            >
                <span class="text-sm font-medium truncate">{name.clone()}</span>
                <span class="flex flex-wrap gap-x-1.5 items-center text-xs text-gray-600 dark:text-gray-300">
                    <Icon icon=icon_for_speed(speed) attr:class="size-3 shrink-0" />
                    <span>{time_control.clone()}</span>
                    <span aria-hidden="true">"·"</span>
                    <span>
                        {move || {
                            t_string!(i18n, tournaments.arena.players_active, count = players)
                                .to_string()
                        }}
                    </span>
                    <span aria-hidden="true">"·"</span>
                    <span class="font-bold tabular-nums text-gray-900 dark:text-gray-100">
                        {move || format_time_left(time_left.get())}
                    </span>
                </span>
            </a>
            <Show
                when=can_join
                fallback=move || {
                    view! {
                        <a
                            href=href.clone()
                            title=move || admission.get().label()
                            class="shrink-0 ui-button ui-button-secondary ui-button-sm"
                        >
                            {t!(i18n, tournaments.arena.view)}
                        </a>
                    }
                }
            >
                <button
                    class="shrink-0 ui-button ui-button-primary ui-button-sm"
                    on:click=join.clone()
                >
                    {t!(i18n, tournaments.arena.join)}
                </button>
            </Show>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_relative_results_are_valid_only_for_the_loaded_identity() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);

        assert!(resource_identity_matches(
            viewer_identity_key(Some(AuthIdentity::Anonymous)),
            viewer_identity_key(Some(AuthIdentity::Anonymous)),
        ));
        assert!(resource_identity_matches(
            viewer_identity_key(Some(AuthIdentity::User(first))),
            viewer_identity_key(Some(AuthIdentity::User(first))),
        ));
        assert!(!resource_identity_matches(
            viewer_identity_key(Some(AuthIdentity::User(second))),
            viewer_identity_key(Some(AuthIdentity::User(first))),
        ));
        assert!(!resource_identity_matches(
            viewer_identity_key(Some(AuthIdentity::Anonymous)),
            viewer_identity_key(Some(AuthIdentity::User(first))),
        ));
        assert!(!resource_identity_matches(
            viewer_identity_key(None),
            viewer_identity_key(Some(AuthIdentity::Anonymous)),
        ));
    }
}
