use super::*;
use shared_types::tournament_view::ArenaPlayerStatsResponse;

use reactive_stores::ArcField;

#[derive(Clone)]
struct ArenaPlayerStatistics {
    performance: Option<(i32, u32)>,
    games: u32,
    win_rate: Option<String>,
    berserk_rate: Option<String>,
    average_opponent: Option<u32>,
}

fn arena_statistics(stats: &ArenaPlayerStatsResponse) -> ArenaPlayerStatistics {
    let games = stats.games_scored;
    ArenaPlayerStatistics {
        performance: stats
            .performance_rating
            .map(|rating| (rating, stats.performance_games)),
        games,
        win_rate: percentage(stats.wins, games),
        berserk_rate: percentage(stats.berserks, games),
        average_opponent: stats.average_opponent_rating,
    }
}

#[derive(Clone, PartialEq)]
struct ArenaHistoryItem {
    opponent: Uuid,
    opponent_rating: Option<u64>,
    color: Color,
    result: String,
    state: Option<&'static str>,
    points: Option<String>,
    own_berserk: bool,
    opponent_berserk: bool,
    game_id: GameId,
}

fn arena_history(arena: Store<ArenaState>, player: Uuid) -> Vec<ArcField<ArenaGameResponse>> {
    let game_ids = arena
        .games()
        .with(|games| games.keys().cloned().collect::<Vec<_>>());
    let mut games = game_ids
        .into_iter()
        .map(|game_id| -> ArcField<ArenaGameResponse> { arena.games().at_key(game_id).into() })
        .filter(|game| game.get_untracked().game.participants.contains(&player))
        .collect::<Vec<_>>();
    games.sort_by_key(|game| game.get_untracked().ordinal);
    games.reverse();
    games
}

fn arena_history_item(
    common: Store<TournamentCommon>,
    configuration: &FormatConfig,
    game: &ArenaGameResponse,
    player: Uuid,
) -> Option<ArenaHistoryItem> {
    let side = game
        .game
        .participants
        .iter()
        .position(|participant| *participant == player)?;
    let opponent = game.game.participants[1 - side];
    let terminal = game.outcome.is_some() || game.game.finished;
    let opponent_rating = game.game.ratings[1 - side].map(u64::from).or_else(|| {
        (!terminal)
            .then(|| {
                common
                    .memberships()
                    .get()
                    .players
                    .get(&opponent)
                    .map(|user| user.rating_for_speed(&game.game.speed))
            })
            .flatten()
    });
    // TODO: i18n once copy is approved.
    let state = match game.outcome {
        Some(GameOutcome::Adjudicated(outcome))
            if matches!(
                (outcome.white(), outcome.black()),
                (
                    AdjudicatedSideResult::DoubleForfeit,
                    AdjudicatedSideResult::DoubleForfeit,
                )
            ) =>
        {
            Some("Double forfeit")
        }
        Some(GameOutcome::Adjudicated(_)) => Some("Adjudicated"),
        Some(GameOutcome::Played(_)) | None => None,
    };
    Some(ArenaHistoryItem {
        opponent,
        opponent_rating,
        color: if side == 0 {
            Color::White
        } else {
            Color::Black
        },
        result: game
            .outcome
            .map(board_result_text)
            .unwrap_or(if game.game.finished { "—" } else { "*" })
            .to_string(),
        state,
        points: game
            .awarded_points
            .map(|points| game_points_text(configuration, points[side])),
        own_berserk: game.game.berserked[side],
        opponent_berserk: game.game.berserked[1 - side],
        game_id: game.game.game_id.clone(),
    })
}

#[component]
fn ArenaPlayerHistoryRow(
    common: Store<TournamentCommon>,
    arena: Store<ArenaState>,
    game: ArcField<ArenaGameResponse>,
    player: Uuid,
    number: usize,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let row_class =
        player_history_row_class("grid-cols-[1.75rem_minmax(0,1fr)_1.25rem_3.25rem_2.25rem]");
    view! {
        {move || {
            let game = game.try_get()?;
            let configuration = FormatConfig::Arena(arena.configuration().get());
            let item = arena_history_item(common, &configuration, &game, player)?;
            let opponent_id = item.opponent;
            let color = item.color;
            let game_id = item.game_id.clone();
            Some(
                view! {
                    <div class=row_class.clone()>
                        <span class="text-xs font-semibold tabular-nums text-center text-gray-500 dark:text-gray-400">
                            {number}
                        </span>
                        <span class="min-w-0">
                            <span class="flex gap-1 items-baseline min-w-0">
                                <button
                                    type="button"
                                    class="min-w-0 font-semibold truncate hover:text-pillbug-teal"
                                    title=move || player_name(
                                        &common.memberships().get(),
                                        opponent_id,
                                    )
                                    on:click=move |_| {
                                        selection
                                            .set(Some(TournamentSelection::Player(opponent_id)))
                                    }
                                >
                                    {move || {
                                        player_name(&common.memberships().get(), opponent_id)
                                    }}
                                </button>
                                {item
                                    .opponent_rating
                                    .map(|rating| {
                                        view! {
                                            <span class="text-xs italic text-gray-500 dark:text-gray-400 shrink-0">
                                                "· " {rating}
                                            </span>
                                        }
                                    })}
                                {item
                                    .opponent_berserk
                                    .then(|| {
                                        view! {
                                            <span class="inline-flex text-[#b46600] dark:text-honeybee-yellow">
                                                <Icon
                                                    icon=icondata_bs::BsLightningFill
                                                    attr:class="size-3"
                                                />
                                            </span>
                                        }
                                    })}
                            </span>
                            {item
                                .state
                                .map(|state| {
                                    view! {
                                        <span class="block text-xs text-gray-500 dark:text-gray-400">
                                            {state}
                                        </span>
                                    }
                                })}
                        </span>
                        <span class="inline-flex justify-center w-5">
                            <span class="inline-flex justify-center w-5 shrink-0">
                                <ColorHex color=Signal::derive(move || color) />
                            </span>
                        </span>
                        // TODO: i18n once copy is approved.
                        <a
                            class="block font-semibold tabular-nums text-right no-link-style hover:text-pillbug-teal"
                            href=format!("/game/{game_id}")
                        >
                            {item.result}
                        </a>
                        <span class="inline-flex gap-0.5 justify-end items-center font-bold tabular-nums text-pillbug-teal">
                            {item
                                .own_berserk
                                .then(|| {
                                    view! {
                                        <span class="inline-flex text-[#b46600] dark:text-honeybee-yellow">
                                            <Icon
                                                icon=icondata_bs::BsLightningFill
                                                attr:class="size-3"
                                            />
                                        </span>
                                    }
                                })} {item.points.unwrap_or_default()}
                        </span>
                    </div>
                },
            )
        }}
    }
}

#[component]
pub(super) fn ArenaPlayerDetailsBody(
    common: Store<TournamentCommon>,
    arena: Store<ArenaState>,
    player: Uuid,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let history = Memo::new_with_compare(move |_| arena_history(arena, player), |_, _| true);

    view! {
        <div class="space-y-3">
            {move || {
                arena
                    .player_stats()
                    .with(|stats| stats.contains_key(&player))
                    .then(|| arena.player_stats().at_key(player).try_get())
                    .flatten()
                    .as_ref()
                    .map(arena_statistics)
                    .map(|statistics| {
                        let ArenaPlayerStatistics {
                            performance,
                            games,
                            win_rate,
                            berserk_rate,
                            average_opponent,
                        } = statistics;
                        view! {
                            <dl class="grid gap-y-1 gap-x-4 px-1 text-sm grid-cols-[minmax(0,1fr)_auto]">
                                {performance
                                    .map(|(performance, performance_games)| {
                                        let value = if performance_games < 3 {
                                            format!("{performance}?")
                                        } else {
                                            performance.to_string()
                                        };
                                        view! {
                                            // TODO: i18n once copy is approved.
                                            <dt class="text-gray-600 dark:text-gray-300">
                                                "Performance"
                                            </dt>
                                            <dd class="font-semibold tabular-nums text-right">
                                                {value}
                                            </dd>
                                        }
                                    })}
                                <dt class="text-gray-600 dark:text-gray-300">
                                    {t!(i18n, tournaments.finished_standings.games)}
                                </dt> <dd class="font-semibold tabular-nums text-right">{games}</dd>
                                {win_rate
                                    .map(|rate| {
                                        view! {
                                            <dt class="text-gray-600 dark:text-gray-300">
                                                {t!(i18n, tournaments.view.arena.win_rate)}
                                            </dt>
                                            <dd class="font-semibold tabular-nums text-right">
                                                {rate}
                                            </dd>
                                        }
                                    })}
                                {berserk_rate
                                    .map(|rate| {
                                        view! {
                                            <dt class="text-gray-600 dark:text-gray-300">
                                                {t!(i18n, tournaments.view.arena.berserk_rate)}
                                            </dt>
                                            <dd class="font-semibold tabular-nums text-right">
                                                {rate}
                                            </dd>
                                        }
                                    })}
                                {average_opponent
                                    .map(|rating| {
                                        view! {
                                            <dt class="text-gray-600 dark:text-gray-300">
                                                {t!(i18n, tournaments.view.arena.average_opponent)}
                                            </dt>
                                            <dd class="font-semibold tabular-nums text-right">
                                                {rating}
                                            </dd>
                                        }
                                    })}
                            </dl>
                        }
                    })
            }}
            <Show
                when=move || history.with(|history| !history.is_empty())
                fallback=move || {
                    view! {
                        <p class="ui-empty-state">
                            {t!(i18n, tournaments.view.standings.no_results)}
                        </p>
                    }
                }
            >
                <div class="rounded">
                    {move || {
                        let history = history.get();
                        let count = history.len();
                        history
                            .into_iter()
                            .enumerate()
                            .map(|(index, game)| {
                                view! {
                                    <ArenaPlayerHistoryRow
                                        common
                                        arena
                                        game
                                        player
                                        number=count - index
                                        selection
                                    />
                                }
                            })
                            .collect_view()
                    }}
                </div>
            </Show>
        </div>
    }
}
