use crate::{
    common::{
        format_tournament_clock,
        ServerMessage,
        ServerResult,
        TournamentClockStyle,
        TournamentUpdate,
    },
    functions::tournaments::get_live_arenas,
    hooks::arena_clock::{format_time_left, use_ticking_now},
    i18n::*,
    providers::{
        websocket::{ConnectionReadyState, WebsocketContext},
        AuthContext,
        AuthIdentity,
    },
    responses::TournamentAbstractResponse,
};
use chrono::{DateTime, Duration, Utc};
use leptos::prelude::*;
use leptos_icons::*;
use leptos_use::{use_document_visibility, use_interval_fn};
use shared_types::{
    clock::Clock,
    tournament::{Format, FormatConfig},
    LIVE_ARENA_LOOKAHEAD,
};
use uuid::Uuid;

type ViewerIdentityKey = (bool, Option<Uuid>);

fn viewer_identity_key(identity: Option<AuthIdentity>) -> ViewerIdentityKey {
    (identity.is_some(), identity.and_then(AuthIdentity::user_id))
}

#[component]
pub fn LiveArenas() -> impl IntoView {
    let i18n = use_i18n();
    let auth = expect_context::<AuthContext>();
    let identity = auth.identity;
    let websocket = expect_context::<WebsocketContext>();
    let arenas = Resource::new(
        move || viewer_identity_key(identity.get()),
        |identity| async move { (identity, get_live_arenas().await) },
    );
    let refresh = move || arenas.refetch();

    let has_opened =
        StoredValue::new(websocket.ready_state.get_untracked() == ConnectionReadyState::Open);
    Effect::watch(
        move || {
            (
                websocket.ready_state.get(),
                websocket.wake_resync_epoch.get(),
            )
        },
        move |(connection, wake), previous, _| {
            let opened = *connection == ConnectionReadyState::Open
                && previous.is_none_or(|(state, _)| *state != ConnectionReadyState::Open);
            let reconnected = opened && has_opened.get_value();
            if opened {
                has_opened.set_value(true);
            }
            let woke = previous.is_some_and(|(_, previous_wake)| previous_wake != wake);
            if reconnected || woke {
                refresh();
            }
        },
        true,
    );

    Effect::watch(
        move || websocket.message.get(),
        move |message, _, _| {
            let Some(ServerResult::Ok(message)) = message else {
                return;
            };
            let ServerMessage::Tournament(update) = message.as_ref() else {
                return;
            };
            let relevant = match update {
                TournamentUpdate::Created {
                    format: Format::Arena,
                    starts_at: Some(start),
                    ..
                } => {
                    let now = Utc::now();
                    *start >= now && *start <= now + LIVE_ARENA_LOOKAHEAD
                }
                TournamentUpdate::Joined(id)
                | TournamentUpdate::Left {
                    tournament_id: id, ..
                }
                | TournamentUpdate::Invited(id)
                | TournamentUpdate::Declined(id)
                | TournamentUpdate::Uninvited(id)
                | TournamentUpdate::OrganizerInvited(id)
                | TournamentUpdate::OrganizerUninvited(id)
                | TournamentUpdate::OrganizerInvitationsClosed(id)
                | TournamentUpdate::OrganizerJoined(id)
                | TournamentUpdate::OrganizerLeft(id, _)
                | TournamentUpdate::Started(id)
                | TournamentUpdate::Finished(id)
                | TournamentUpdate::Deleted(id) => arenas.with_untracked(|response| {
                    response.as_ref().is_some_and(|(_, result)| {
                        result.as_ref().is_ok_and(|arenas| {
                            arenas.iter().any(|arena| arena.tournament_id == *id)
                        })
                    })
                }),
                _ => false,
            };
            if relevant {
                refresh();
            }
        },
        false,
    );

    let visibility = use_document_visibility();
    use_interval_fn(
        move || {
            if visibility.get_untracked() == web_sys::VisibilityState::Visible {
                refresh();
            }
        },
        300_000,
    );
    let now = use_ticking_now();
    let expanded = RwSignal::new(false);

    view! {
        <Transition>
            {move || {
                let current_identity = viewer_identity_key(identity.get());
                arenas
                    .get()
                    .filter(|(loaded_identity, _)| *loaded_identity == current_identity)
                    .and_then(|(_, result)| result.ok())
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
                        let has_arenas = !arenas.is_empty();
                        let hidden_count = arenas.len().saturating_sub(3);

                        // Countdown ticks only update the cards. Selection changes
                        // come from a successful refresh, including after expiry.
                        view! {
                            <Show when=move || has_arenas>
                                <section class="overflow-hidden mx-auto w-full ui-panel">
                                    <header class="py-3 px-4">
                                        <h2 class="text-sm font-semibold text-gray-600 dark:text-gray-300">

                                            {move || {
                                                t_string!(i18n, tournaments.ui.arenas).to_string()
                                            }}
                                        </h2>
                                    </header>
                                    <ul
                                        id="live-arenas-list"
                                        class="border-t divide-y border-black/10 divide-black/10 dark:border-white/10 dark:divide-white/10"
                                    >
                                        {
                                            let arenas = arenas.clone();
                                            move || {
                                                arenas
                                                    .clone()
                                                    .into_iter()
                                                    .take(if expanded.get() { usize::MAX } else { 3 })
                                                    .map(|(arena, ends_at)| {
                                                        view! { <ArenaCard arena=arena now ends_at /> }
                                                    })
                                                    .collect_view()
                                            }
                                        }
                                    </ul>
                                    <Show when=move || { hidden_count > 0 }>
                                        <div class="py-2 px-4 text-right border-t border-black/10 dark:border-white/10">

                                            <button
                                                type="button"
                                                class="text-xs ui-text-link"
                                                aria-expanded=expanded
                                                aria-controls="live-arenas-list"
                                                on:click=move |_| expanded.update(|value| *value = !*value)
                                            >
                                                {move || {
                                                    if expanded.get() {
                                                        t_string!(i18n, common.show_fewer).to_string()
                                                    } else {
                                                        t_string!(
                                                            i18n, tournaments.ui.more_arenas, hidden_count = hidden_count
                                                        )
                                                            .to_string()
                                                    }
                                                }}
                                            </button>
                                        </div>
                                    </Show>
                                </section>
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
    let href = format!("/tournament/{}", arena.tournament_id.0);
    let name = arena.name.clone();
    let players = arena.players;
    let starts_at = arena.scheduled_starts_at();
    let arena_configuration = match &arena.configuration.format {
        FormatConfig::Arena(configuration) => Some(configuration),
        _ => None,
    };
    let time_control = arena_configuration.map_or_else(String::new, |configuration| {
        format_tournament_clock(
            i18n,
            Clock::Realtime(configuration.game_clock),
            TournamentClockStyle::Compact,
        )
    });
    let upcoming = Signal::derive(move || starts_at.is_some_and(|start| now.get() < start));
    let time_left = Signal::derive(move || {
        if upcoming.get() {
            starts_at.unwrap_or(ends_at) - now.get()
        } else {
            ends_at - now.get()
        }
    });
    let countdown_label = move || {
        if upcoming.get() {
            t_string!(i18n, tournaments.ui.starts_in).to_string()
        } else if now.get() >= ends_at {
            t_string!(i18n, tournaments.ui.ending).to_string()
        } else {
            t_string!(i18n, tournaments.ui.remaining).to_string()
        }
    };

    view! {
        <li>
            <a
                href=href
                class="flex gap-3 justify-between items-center py-3 px-4 no-link-style group dark:hover:bg-pillbug-teal/10 hover:bg-blue-light/40 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-pillbug-teal"
            >
                <span class="flex flex-col gap-1 min-w-0 grow">
                    <span class="text-sm font-semibold text-gray-900 transition-colors dark:text-gray-100 wrap-break-word group-hover:text-pillbug-teal">
                        {name}
                    </span>
                    <span class="text-xs text-gray-600 dark:text-gray-400">

                        {time_control}" · "
                        {move || {
                            format!(
                                "{players} {}",
                                if players == 1 {
                                    t_string!(i18n, tournaments.ui.player).to_string()
                                } else {
                                    t_string!(i18n, tournaments.ui.players).to_string()
                                },
                            )
                        }}
                    </span>
                </span>
                <span class="flex flex-col gap-1 items-end shrink-0">
                    <span class="text-xs text-gray-500 dark:text-gray-400">{countdown_label}</span>
                    <time
                        class="text-sm font-semibold tabular-nums text-gray-700 dark:text-gray-200"
                        aria-label=move || {
                            format!(
                                "{} {}",
                                countdown_label(),
                                format_time_left(i18n, time_left.get()),
                            )
                        }
                    >
                        {move || format_time_left(i18n, time_left.get())}
                    </time>
                </span>
                <Icon
                    icon=icondata_lu::LuChevronRight
                    attr:class="size-3.5 shrink-0 text-gray-400"
                    attr:aria-hidden="true"
                />
            </a>
        </li>
    }
}
