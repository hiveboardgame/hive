use crate::{
    common::TournamentAction,
    components::{atoms::rating::icon_for_speed, molecules::panel::Panel},
    functions::tournaments::get_by_status,
    hooks::arena_clock::{format_time_left, use_ticking_now},
    providers::{ApiRequestsProvider, AuthContext, UpdateNotifier},
    responses::TournamentAbstractResponse,
};
use chrono::Duration;
use leptos::prelude::*;
use leptos_icons::*;
use leptos_router::hooks::use_navigate;
use shared_types::{GameSpeed, TournamentSortOrder, TournamentStatus};

/// Arenas running right now, for the front page.
///
/// An arena is the one format worth advertising while it is under way: it takes
/// newcomers for as long as its clock runs, so somebody arriving at the site
/// mid-arena can still join and be paired on the next tick.
#[component]
pub fn LiveArenas() -> impl IntoView {
    // Refetched whenever any tournament update lands, which includes the global
    // `ArenaStarted` — an arena that opens while somebody is sitting on the front
    // page has to appear without them reloading.
    let update = expect_context::<UpdateNotifier>().tournament_update;
    let arenas = Resource::new(
        move || update.get(),
        |_| {
            get_by_status(
                TournamentStatus::InProgress,
                TournamentSortOrder::StartedAtDesc,
            )
        },
    );

    let now = use_ticking_now();

    // Filtered on the clock rather than on status alone, so the last arena's
    // expiry takes the whole panel away instead of leaving an empty heading: a
    // finished arena stays `InProgress` until the job gets to it.
    let live = move || {
        let now = now.get();
        arenas
            .get()
            .and_then(Result::ok)
            .map(|tournaments| {
                tournaments
                    .into_iter()
                    .filter(|tournament| {
                        tournament.mode == "Arena"
                            && tournament
                                .started_at
                                .zip(tournament.arena_duration_seconds)
                                .is_some_and(|(started_at, duration)| {
                                    started_at + Duration::seconds(duration as i64) > now
                                })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };

    view! {
        <Transition>
            {move || {
                let arenas = live();
                (!arenas.is_empty())
                    .then(|| {
                        view! {
                            // Exactly what `Challenges` constrains itself to
                            // (`challenges.rs:283`), so the two panels line up
                            // instead of one guessing at the other's width.
                            <div class="mx-auto w-full max-w-screen-md">
                                <Panel title="Arenas running now">
                                    <ul class="divide-y divide-gray-200 dark:divide-gray-700">
                                        {arenas
                                            .into_iter()
                                            .map(|arena| view! { <ArenaCard arena /> })
                                            .collect_view()}
                                    </ul>
                                </Panel>
                            </div>
                        }
                    })
            }}
        </Transition>
    }
}

#[component]
fn ArenaCard(arena: TournamentAbstractResponse) -> impl IntoView {
    let auth_context = expect_context::<AuthContext>();
    let api = expect_context::<ApiRequestsProvider>().0;

    let tournament_id = arena.tournament_id.clone();
    let href = format!("/tournament/{}", tournament_id.0);
    let name = arena.name.clone();
    let players = arena.players;
    let player_list = arena.player_list.clone();
    let ends_at = arena
        .started_at
        .zip(arena.arena_duration_seconds)
        .map(|(started_at, duration)| started_at + Duration::seconds(duration as i64));
    let speed = GameSpeed::from_base_increment(arena.time_base, arena.time_increment);
    let time_control = match (arena.time_base, arena.time_increment) {
        (Some(base), Some(increment)) => format!("{}+{increment}", base / 60),
        _ => String::new(),
    };

    let now = use_ticking_now();
    let time_left = Signal::derive(move || ends_at.map(|ends_at| ends_at - now.get()));

    // Somebody already in the pool wants the arena page, not another join: pause
    // and leave only exist there.
    let is_entrant = Signal::derive(move || {
        auth_context.user.with(|user| {
            user.as_ref()
                .is_some_and(|user| player_list.contains(&user.id))
        })
    });
    let is_signed_in = Signal::derive(move || auth_context.user.with(Option::is_some));

    let join = {
        let href = href.clone();
        move |_| {
            api.get()
                .tournament(TournamentAction::JoinArena(tournament_id.clone()));
            use_navigate()(&href, Default::default());
        }
    };

    view! {
        <li class="flex gap-3 justify-between items-center py-2 px-1">
            <a
                href=href.clone()
                class="flex flex-col min-w-0 rounded transition-opacity hover:opacity-80 grow"
            >
                <span class="text-sm font-medium truncate">{name.clone()}</span>
                // Wraps rather than overflows: on a narrow phone the speed, the
                // field size and the countdown together outrun one line.
                <span class="flex flex-wrap gap-x-1.5 items-center text-xs text-gray-600 dark:text-gray-300">
                    <Icon icon=icon_for_speed(speed) attr:class="size-3 shrink-0" />
                    <span>{time_control.clone()}</span>
                    <span aria-hidden="true">"·"</span>
                    <span>{format!("{players} playing")}</span>
                    <span aria-hidden="true">"·"</span>
                    <span class="font-bold tabular-nums text-gray-900 dark:text-gray-100">
                        {move || time_left.get().map(format_time_left)}
                    </span>
                </span>
            </a>
            <Show
                when=move || is_signed_in.get() && !is_entrant.get()
                fallback=move || {
                    view! {
                        <a
                            href=href.clone()
                            class="shrink-0 ui-button ui-button-secondary ui-button-sm"
                        >
                            "View"
                        </a>
                    }
                }
            >
                <button
                    class="shrink-0 ui-button ui-button-primary ui-button-sm"
                    on:click=join.clone()
                >
                    "Join"
                </button>
            </Show>
        </li>
    }
}
