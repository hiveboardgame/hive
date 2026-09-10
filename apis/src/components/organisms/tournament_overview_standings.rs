use crate::{
    common::{
        display_standing_rows,
        primary_score_presentation,
        standings_value_text,
        ScorePresentation,
    },
    components::{
        molecules::tournament_standings_controls::TournamentStandingsControls,
        organisms::{
            elimination_result::overview_result_label,
            tournament_inspector::TournamentSelection,
            tournament_podium::TournamentPodium,
        },
    },
    i18n::*,
    providers::{
        ArenaState,
        ArenaStateStoreFields,
        AuthContext,
        AuthIdentity,
        EliminationStateStoreFields,
        RoundRobinState,
        RoundRobinStateStoreFields,
        SwissState,
        SwissStateStoreFields,
        TournamentCommonStoreFields,
        TournamentFormatStore,
        TournamentState,
    },
    responses::{
        tournament::rating_clock,
        TournamentLifecycleDetails,
        TournamentMemberships,
        TournamentStandings,
    },
};
use leptos::prelude::*;
use leptos_icons::Icon;
use reactive_stores::{ArcField, Store};
use shared_types::{
    tournament::{AdjudicatedSideResult, FormatConfig, GameOutcome, PlayedGameOutcome},
    tournament_view::{
        ArenaGameResponse,
        ArenaPlayerStatsResponse,
        EliminationPlayerResultResponse,
        SlotResponse,
    },
    GameSpeed,
    TournamentStatus,
};
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    fmt,
};
use tournamint::MatchScore;
use uuid::Uuid;

const PAGE_SIZE: usize = 10;

#[derive(Clone)]
struct OverviewRow {
    player: Uuid,
    name: String,
    rating: Option<u64>,
    rank: Option<u32>,
    score: Option<String>,
    state: Option<String>,
    faded: bool,
    withdrawn: bool,
    arena: Option<ArenaRowDetails>,
}

#[derive(Clone)]
struct OverviewStandingsModel {
    rows: Vec<OverviewRow>,
    active_total: usize,
    struck_total: Option<usize>,
}

#[derive(Clone)]
struct ArenaRowDetails {
    stats: Option<ArcField<ArenaPlayerStatsResponse>>,
    games: Vec<ArcField<ArenaGameResponse>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArenaResult {
    Win,
    Draw,
    Loss,
    Active,
    Unscored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScoreEmphasis {
    Regular,
    Win,
    Doubled,
    Active,
    Unscored,
}

impl ScoreEmphasis {
    const fn class(self) -> &'static str {
        match self {
            Self::Regular => "text-gray-500 dark:text-gray-400",
            Self::Win => "font-semibold text-pillbug-teal",
            Self::Doubled => "font-bold text-[#b46600] dark:text-honeybee-yellow",
            Self::Active => "font-bold text-pillbug-teal animate-pulse",
            Self::Unscored => "text-gray-400 dark:text-gray-500",
        }
    }
}

#[derive(Clone, Debug)]
struct ArenaGameView {
    result: ArenaResult,
    ordinal: i64,
    awarded_points: Option<u32>,
    doubled: bool,
}

#[derive(Clone)]
struct ArenaScoreView {
    text: String,
    emphasis: ScoreEmphasis,
}

#[derive(Clone, Copy)]
enum ArenaRating {
    Tournament(i32),
    Account(u64),
}

impl fmt::Display for ArenaRating {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tournament(rating) => rating.fmt(formatter),
            Self::Account(rating) => rating.fmt(formatter),
        }
    }
}

fn arena_result(game: &ArenaGameResponse, side: usize) -> ArenaResult {
    if !game.game.finished {
        return ArenaResult::Active;
    }
    match game.outcome {
        Some(GameOutcome::Played(PlayedGameOutcome::Draw)) => ArenaResult::Draw,
        Some(GameOutcome::Played(PlayedGameOutcome::WhiteWin)) => {
            if side == 0 {
                ArenaResult::Win
            } else {
                ArenaResult::Loss
            }
        }
        Some(GameOutcome::Played(PlayedGameOutcome::BlackWin)) => {
            if side == 1 {
                ArenaResult::Win
            } else {
                ArenaResult::Loss
            }
        }
        Some(GameOutcome::Adjudicated(outcome)) => match if side == 0 {
            outcome.white()
        } else {
            outcome.black()
        } {
            AdjudicatedSideResult::ForfeitWin => ArenaResult::Win,
            AdjudicatedSideResult::Draw => ArenaResult::Draw,
            AdjudicatedSideResult::ForfeitLoss | AdjudicatedSideResult::DoubleForfeit => {
                ArenaResult::Loss
            }
        },
        None => ArenaResult::Unscored,
    }
}

fn arena_player_game_views(
    games: &[ArcField<ArenaGameResponse>],
    player: Uuid,
) -> Vec<ArenaGameView> {
    let mut games = games
        .iter()
        .filter_map(|game| {
            let game = game.try_get()?;
            let side = game
                .game
                .participants
                .iter()
                .position(|participant| *participant == player)?;
            Some(ArenaGameView {
                result: arena_result(&game, side),
                ordinal: game.ordinal,
                awarded_points: game.awarded_points.map(|points| points[side].value()),
                doubled: game.doubled.is_some_and(|doubled| doubled[side]),
            })
        })
        .collect::<Vec<_>>();
    games.sort_by_key(|game| game.ordinal);
    games
}

fn arena_score_views(games: &[ArenaGameView]) -> Vec<ArenaScoreView> {
    games
        .iter()
        .map(|game| {
            let emphasis = match game.result {
                ArenaResult::Win | ArenaResult::Draw if game.doubled => ScoreEmphasis::Doubled,
                ArenaResult::Win => ScoreEmphasis::Win,
                ArenaResult::Draw | ArenaResult::Loss => ScoreEmphasis::Regular,
                ArenaResult::Active => ScoreEmphasis::Active,
                ArenaResult::Unscored => ScoreEmphasis::Unscored,
            };
            let text = match (game.result, game.awarded_points) {
                (ArenaResult::Active, _) => String::from("•"),
                (_, Some(points)) => points.to_string(),
                (_, None) => String::from("–"),
            };
            ArenaScoreView { text, emphasis }
        })
        .collect()
}

fn arena_games_by_player(
    arena: Store<ArenaState>,
) -> HashMap<Uuid, Vec<ArcField<ArenaGameResponse>>> {
    let game_ids = arena
        .games()
        .with(|games| games.keys().cloned().collect::<Vec<_>>());
    let mut games = HashMap::<Uuid, Vec<ArcField<ArenaGameResponse>>>::new();
    for game_id in game_ids {
        let game: ArcField<ArenaGameResponse> = arena.games().at_key(game_id).into();
        for player in game.get_untracked().game.participants {
            games.entry(player).or_default().push(game.clone());
        }
    }
    games
}

fn outcome_marker(slot: &SlotResponse, player: Uuid) -> Option<&'static str> {
    let side = slot
        .participants
        .iter()
        .position(|participant| *participant == player)?;
    let outcome = slot.outcome()?;
    let score = if side == 0 {
        outcome.white().score()
    } else {
        outcome.black().score()
    };
    Some(match score {
        MatchScore::Win => "W",
        MatchScore::Draw => "D",
        MatchScore::Loss => "L",
    })
}

fn slot_marker(slot: &SlotResponse, player: Uuid) -> &'static str {
    outcome_marker(slot, player).unwrap_or_else(|| {
        if slot.game.is_some() && slot.resolution.is_none() {
            "*"
        } else {
            "–"
        }
    })
}

fn round_robin_results(format: Store<RoundRobinState>, player: Uuid) -> Vec<&'static str> {
    let rounds = format.rounds().get();
    rounds
        .iter()
        .filter_map(|round| {
            if round.resting == Some(player) {
                return Some("–");
            }
            round.slots.iter().find_map(|slot| {
                let slot = format.slots().at_key(slot.slot_id);
                slot.get_untracked()
                    .participants
                    .contains(&player)
                    .then(|| slot_marker(&slot.get(), player))
            })
        })
        .collect()
}

fn swiss_results(
    format: Store<SwissState>,
    status: TournamentStatus,
    player: Uuid,
) -> Vec<&'static str> {
    let rounds = format.rounds().get();
    let projected_round_count = rounds
        .last()
        .map(|round| round.round_index as usize)
        .map_or(0, |round| round.saturating_add(1));
    let round_count = if status == TournamentStatus::Finished {
        projected_round_count
    } else {
        format
            .configuration()
            .get_untracked()
            .rounds
            .resolved_rounds()
            .map_or(projected_round_count, |rounds| rounds.get() as usize)
    };
    (0..round_count)
        .map(|round_index| {
            let Some(round) = rounds
                .iter()
                .find(|round| round.round_index as usize == round_index)
            else {
                return "–";
            };
            if round.byes.iter().any(|bye_| bye_.player == player) {
                return "B";
            }
            let Some(encounter) = round
                .encounters
                .iter()
                .find(|encounter| encounter.participants.contains(&player))
            else {
                return "–";
            };
            let side = usize::from(encounter.participants[1] == player);
            if let Some(completion) = encounter.completion {
                return match completion.aggregate[side] {
                    MatchScore::Win => "W",
                    MatchScore::Draw => "D",
                    MatchScore::Loss => "L",
                };
            }
            if encounter.slot_ids.len() == 1 {
                return slot_marker(&format.slots().at_key(encounter.slot_ids[0]).get(), player);
            }
            if encounter.slot_ids.iter().any(|slot_id| {
                let slot = format.slots().at_key(*slot_id).get();
                slot.game.is_some() && slot.resolution.is_none()
            }) {
                "*"
            } else {
                "–"
            }
        })
        .collect()
}

fn compact_results(
    format: TournamentFormatStore,
    status: TournamentStatus,
    player: Uuid,
) -> Vec<&'static str> {
    match format {
        TournamentFormatStore::Swiss(format) => swiss_results(format, status, player),
        TournamentFormatStore::RoundRobin(format) => round_robin_results(format, player),
        TournamentFormatStore::Arena(_) | TournamentFormatStore::Elimination(_) => Vec::new(),
    }
}

fn result_marker_color(marker: &str) -> &'static str {
    match marker {
        "W" => "text-grasshopper-green",
        "D" => "text-pillbug-teal",
        "L" => "text-ladybug-red",
        _ => "text-gray-500 dark:text-gray-400",
    }
}

fn format_configuration(format: TournamentFormatStore) -> FormatConfig {
    match format {
        TournamentFormatStore::Arena(state) => {
            FormatConfig::Arena(state.configuration().get_untracked())
        }
        TournamentFormatStore::RoundRobin(state) => {
            FormatConfig::RoundRobin(state.configuration().get_untracked())
        }
        TournamentFormatStore::Swiss(state) => {
            FormatConfig::Swiss(state.configuration().get_untracked())
        }
        TournamentFormatStore::Elimination(state) => {
            FormatConfig::Elimination(state.configuration().get_untracked())
        }
    }
}

fn compact_rows(
    lifecycle: &TournamentLifecycleDetails,
    memberships: &TournamentMemberships,
    standings: &TournamentStandings,
    format: TournamentFormatStore,
    configuration: &FormatConfig,
) -> Vec<OverviewRow> {
    let speed = rating_clock(configuration).map(GameSpeed::from);
    if lifecycle.status == TournamentStatus::NotStarted {
        let mut players = memberships.players.values().cloned().collect::<Vec<_>>();
        players.sort_by_cached_key(|user| {
            (
                Reverse(
                    speed
                        .as_ref()
                        .map(|speed| user.rating_for_speed(speed))
                        .unwrap_or_default(),
                ),
                user.username.to_lowercase(),
                user.uid,
            )
        });
        return players
            .into_iter()
            .zip(1_u32..)
            .map(|(user, rank)| OverviewRow {
                rating: speed.as_ref().map(|speed| user.rating_for_speed(speed)),
                player: user.uid,
                name: user.username,
                rank: Some(rank),
                score: None,
                state: None,
                faded: false,
                withdrawn: false,
                arena: None,
            })
            .collect();
    }

    if let TournamentFormatStore::Arena(arena) = format {
        let indexed_games = arena_games_by_player(arena);
        let stat_players = arena
            .player_stats()
            .with(|stats| stats.keys().copied().collect::<HashSet<_>>());
        let mut rows = standings
            .snapshot
            .iter()
            .flat_map(|snapshot| &snapshot.groups)
            .flat_map(|group| {
                group.rows.iter().filter_map(|standing| {
                    let user = memberships.players.get(&standing.user_id)?;
                    Some(OverviewRow {
                        player: user.uid,
                        name: user.username.clone(),
                        rating: speed.as_ref().map(|speed| user.rating_for_speed(speed)),
                        rank: Some(group.placement.rank()),
                        score: Some(standings_value_text(
                            standing.primary_score,
                            ScorePresentation::ArenaPoints,
                        )),
                        state: None,
                        faded: false,
                        withdrawn: false,
                        arena: Some(ArenaRowDetails {
                            stats: stat_players
                                .contains(&user.uid)
                                .then(|| arena.player_stats().at_key(user.uid).into()),
                            games: indexed_games.get(&user.uid).cloned().unwrap_or_default(),
                        }),
                    })
                })
            })
            .collect::<Vec<_>>();
        let ranked = rows.iter().map(|row| row.player).collect::<HashSet<_>>();
        let mut unranked = memberships
            .players
            .values()
            .filter(|user| !ranked.contains(&user.uid))
            .cloned()
            .collect::<Vec<_>>();
        unranked.sort_by_cached_key(|user| (user.username.to_lowercase(), user.uid));
        let ranked_count = rows.len();
        rows.extend(
            (1_u32..)
                .skip(ranked_count)
                .zip(unranked)
                .map(|(rank, user)| {
                    let player = user.uid;
                    let rating = speed.as_ref().map(|speed| user.rating_for_speed(speed));
                    OverviewRow {
                        player,
                        name: user.username,
                        rating,
                        rank: Some(rank),
                        score: Some(String::from("0")),
                        state: None,
                        faded: false,
                        withdrawn: false,
                        arena: Some(ArenaRowDetails {
                            stats: stat_players
                                .contains(&player)
                                .then(|| arena.player_stats().at_key(player).into()),
                            games: indexed_games.get(&player).cloned().unwrap_or_default(),
                        }),
                    }
                }),
        );
        return rows;
    }

    if matches!(configuration, FormatConfig::Elimination(_)) {
        let TournamentFormatStore::Elimination(format) = format else {
            unreachable!("Elimination configuration has an Elimination response")
        };
        let mut results = format.player_results().get();
        sort_elimination_results(&mut results);
        return results
            .iter()
            .copied()
            .filter_map(|result| {
                let user = memberships.players.get(&result.player)?;
                let rating = speed.as_ref().map(|speed| user.rating_for_speed(speed));
                Some(OverviewRow {
                    player: user.uid,
                    name: user.username.clone(),
                    rating,
                    rank: result.placement,
                    score: None,
                    state: overview_result_label(result),
                    faded: result.placement.is_some_and(|place| place != 1),
                    withdrawn: memberships.withdrawn.contains(&user.uid),
                    arena: None,
                })
            })
            .collect();
    }

    standings
        .snapshot
        .iter()
        .flat_map(|snapshot| display_standing_rows(snapshot, &memberships.withdrawn))
        .filter_map(|standing| {
            let user = memberships.players.get(&standing.row.user_id)?;
            Some(OverviewRow {
                player: user.uid,
                name: user.username.clone(),
                rating: speed.as_ref().map(|speed| user.rating_for_speed(speed)),
                rank: standing.rank,
                score: Some(standings_value_text(
                    standing.row.primary_score,
                    primary_score_presentation(configuration),
                )),
                state: None,
                faded: false,
                withdrawn: standing.withdrawn,
                arena: None,
            })
        })
        .collect()
}

fn sort_elimination_results(results: &mut [EliminationPlayerResultResponse]) {
    results.sort_by_key(|result| (!result.in_contention, result.placement.unwrap_or(u32::MAX)));
}

fn active_player_total(
    lifecycle: &TournamentLifecycleDetails,
    memberships: &TournamentMemberships,
    format: TournamentFormatStore,
) -> usize {
    if lifecycle.status == TournamentStatus::NotStarted {
        return memberships.players.len();
    }
    match format {
        TournamentFormatStore::RoundRobin(_) | TournamentFormatStore::Swiss(_) => memberships
            .players
            .keys()
            .filter(|player| !memberships.withdrawn.contains(player))
            .count(),
        TournamentFormatStore::Elimination(format) => format
            .player_results()
            .get()
            .iter()
            .filter(|result| result.in_contention)
            .count(),
        TournamentFormatStore::Arena(_) => memberships.players.len(),
    }
}

impl OverviewStandingsModel {
    fn read(tournament: TournamentState) -> Self {
        let lifecycle = tournament.common.lifecycle().get();
        let memberships = tournament.common.memberships().get();
        let standings = tournament.common.standings().get();
        let configuration = format_configuration(tournament.format);
        let active_total = active_player_total(&lifecycle, &memberships, tournament.format);
        let player_total = memberships.players.len();
        Self {
            rows: compact_rows(
                &lifecycle,
                &memberships,
                &standings,
                tournament.format,
                &configuration,
            ),
            active_total,
            struck_total: (lifecycle.status != TournamentStatus::NotStarted
                && active_total < player_total)
                .then_some(player_total),
        }
    }
}

fn maximum_result_count(format: TournamentFormatStore, status: TournamentStatus) -> usize {
    match format {
        TournamentFormatStore::Arena(_) => 0,
        TournamentFormatStore::RoundRobin(round_robin) => round_robin.rounds().get().len(),
        TournamentFormatStore::Swiss(swiss) => {
            let rounds = swiss.rounds().get();
            let projected = rounds
                .last()
                .map(|round| round.round_index as usize + 1)
                .unwrap_or_default();
            if status == TournamentStatus::Finished {
                projected
            } else {
                swiss
                    .configuration()
                    .get_untracked()
                    .rounds
                    .resolved_rounds()
                    .map_or(projected, |rounds| rounds.get() as usize)
            }
        }
        TournamentFormatStore::Elimination(_) => 0,
    }
}

#[component]
fn StandingPlayerIdentity(
    name: String,
    rating: Signal<Option<String>>,
    withdrawn: bool,
    fixed_desktop_width: bool,
    inline_desktop_rating: bool,
    show_missing_rating: bool,
) -> impl IntoView {
    let class = if fixed_desktop_width {
        "flex flex-1 flex-col justify-center py-2 px-1 min-w-0 text-left sm:w-40 sm:flex-none"
    } else {
        "flex flex-1 flex-col justify-center py-2 px-1 min-w-0 text-left"
    };
    let mobile_rating = rating;
    let desktop_rating = rating;
    view! {
        <div class=class>
            <div class="flex items-baseline min-w-0">
                <span class=if withdrawn {
                    "block flex-1 min-w-0 font-semibold line-through truncate"
                } else {
                    "block flex-1 min-w-0 font-semibold truncate"
                }>{name}</span>
                {inline_desktop_rating
                    .then(|| {
                        view! {
                            <span class="hidden ml-1 text-xs italic text-gray-500 whitespace-nowrap sm:inline dark:text-gray-400 shrink-0">
                                {move || desktop_rating.get()}
                            </span>
                        }
                    })}
            </div>
            {move || {
                mobile_rating
                    .get()
                    .or_else(|| show_missing_rating.then(|| String::from("—")))
                    .map(|rating| {
                        view! {
                            <span class="text-xs italic tabular-nums text-gray-500 sm:hidden dark:text-gray-400 truncate">
                                {rating}
                            </span>
                        }
                    })
            }}
        </div>
    }
}

#[component]
fn ArenaStandingRow(
    tournament: TournamentState,
    row: OverviewRow,
    is_me: bool,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let player = row.player;
    let rank = row.rank;
    let name = row.name;
    let account_rating = row.rating;
    let score = row.score.unwrap_or_else(|| String::from("0"));
    let details = row.arena.expect("Arena row has Arena details");
    let pause_stats = details.stats.clone();
    let rating_stats = details.stats.clone();
    let fire_stats = details.stats;
    let games = details.games;
    let finished = Signal::derive(move || {
        tournament.common.lifecycle().get().status == TournamentStatus::Finished
    });
    let rating = Signal::derive(move || {
        rating_stats
            .as_ref()
            .and_then(|stats| stats.try_get())
            .and_then(|stats| stats.arena_rating)
            .map(ArenaRating::Tournament)
            .or_else(|| account_rating.map(ArenaRating::Account))
            .map(|rating| rating.to_string())
    });
    view! {
        <div
            class=move || {
                if selection.get() == Some(TournamentSelection::Player(player)) {
                    "flex flex-wrap items-center w-full ui-overview-standing-row !bg-pillbug-teal/25 cursor-pointer"
                } else {
                    "flex flex-wrap items-center w-full ui-overview-standing-row cursor-pointer"
                }
            }
            tabindex=0
            data-tournament-overview-standing="true"
            data-tournament-player-id=player.to_string()
            on:click=move |_| selection.set(Some(TournamentSelection::Player(player)))
            on:keydown=move |event| {
                if event.key() == "Enter" || event.key() == " " {
                    event.prevent_default();
                    selection.set(Some(TournamentSelection::Player(player)));
                }
            }
        >
            <div class=if is_me {
                "flex flex-none justify-center items-center py-2 px-1 w-6 font-bold tabular-nums text-gray-600 border-l-4 border-pillbug-teal sm:w-8 dark:text-gray-300"
            } else {
                "flex flex-none justify-center items-center py-2 px-1 w-6 font-bold tabular-nums text-gray-600 sm:w-8 dark:text-gray-300"
            }>
                {move || {
                    if !finished.get()
                        && pause_stats
                            .as_ref()
                            .is_some_and(|stats| stats.try_get().is_some_and(|stats| stats.paused))
                    {
                        view! {
                            <span
                                class="inline-flex justify-center items-center text-gray-400"
                                title=move || t_string!(i18n, tournaments.arena.pause).to_string()
                            >
                                <Icon icon=icondata_bs::BsPauseCircleFill attr:class="size-4" />
                            </span>
                        }
                            .into_any()
                    } else {
                        view! {
                            <span>
                                {rank
                                    .map(|rank| rank.to_string())
                                    .unwrap_or_else(|| String::from("—"))}
                            </span>
                        }
                            .into_any()
                    }
                }}
            </div>
            <StandingPlayerIdentity
                name
                rating
                withdrawn=false
                fixed_desktop_width=true
                inline_desktop_rating=true
                show_missing_rating=false
            />
            <div class="order-last py-1 pr-2 pl-6 min-w-0 text-right sm:flex-1 sm:order-none sm:py-2 sm:px-2 basis-full sm:basis-auto">
                {move || {
                    let scores = arena_score_views(&arena_player_game_views(&games, player));
                    let sheet_size = match scores.len() {
                        81.. => "text-[0.7rem] tracking-tight",
                        36..=80 => "text-xs tracking-tight",
                        _ => "text-sm tracking-wide",
                    };
                    if scores.is_empty() {
                        view! {
                            <span class="font-mono text-gray-400 dark:text-gray-500">"—"</span>
                        }
                            .into_any()
                    } else {
                        view! {
                            <span class=format!(
                                "flex w-full flex-wrap items-center justify-end gap-1 leading-tight font-mono {sheet_size}",
                            )>
                                {scores
                                    .into_iter()
                                    .map(|score| {
                                        view! {
                                            <span class=format!(
                                                "shrink-0 {}",
                                                score.emphasis.class(),
                                            )>{score.text}</span>
                                        }
                                    })
                                    .collect_view()}
                            </span>
                        }
                            .into_any()
                    }
                }}
            </div>
            <div class="flex flex-none gap-1 justify-end items-center py-2 px-1 text-lg font-bold tabular-nums text-right whitespace-nowrap sm:px-3 min-w-8 sm:min-w-14">
                {move || {
                    (!finished.get()
                        && fire_stats
                            .as_ref()
                            .is_some_and(|stats| {
                                stats.try_get().is_some_and(|stats| stats.on_fire)
                            }))
                        .then(|| {
                            view! {
                                <span
                                    class="text-[#b46600] dark:text-honeybee-yellow"
                                    title=move || {
                                        t_string!(i18n, tournaments.view.arena.on_fire).to_string()
                                    }
                                >
                                    <Icon icon=icondata_bs::BsFire attr:class="size-4" />
                                </span>
                            }
                        })
                }} <span>{score.clone()}</span>
            </div>
        </div>
    }
}

#[component]
fn StandardStandingRow(
    tournament: TournamentState,
    row: OverviewRow,
    is_me: bool,
    show_results: bool,
    show_rating: bool,
    show_final: bool,
    interactive: bool,
    max_result_count: Signal<usize>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let player = row.player;
    let elimination = matches!(tournament.format, TournamentFormatStore::Elimination(_));
    let wide_state = row.state.is_some();
    let final_value = row
        .state
        .clone()
        .or(row.score.clone())
        .unwrap_or_else(|| String::from("—"));
    let seed = row.rank;
    let name = row.name;
    let account_rating = row.rating;
    let rating = Signal::derive(move || account_rating.map(|rating| rating.to_string()));
    let faded = row.faded;
    let withdrawn = row.withdrawn;
    let class = move || {
        if selection.get() == Some(TournamentSelection::Player(player)) {
            if interactive {
                "flex flex-wrap items-center w-full ui-overview-standing-row !bg-pillbug-teal/25 cursor-pointer"
            } else {
                "flex flex-wrap items-center w-full ui-overview-standing-row !bg-pillbug-teal/25"
            }
        } else if !interactive {
            "flex flex-wrap items-center w-full ui-overview-standing-row"
        } else if faded {
            "flex flex-wrap items-center w-full ui-overview-standing-row opacity-55 cursor-pointer"
        } else {
            "flex flex-wrap items-center w-full ui-overview-standing-row cursor-pointer"
        }
    };
    let results_expanded = RwSignal::new(false);
    let results = Signal::derive(move || {
        compact_results(
            tournament.format,
            tournament.common.lifecycle().get().status,
            player,
        )
    });
    view! {
        <div
            class=class
            tabindex=interactive.then_some(0)
            data-tournament-overview-standing="true"
            data-tournament-player-id=player.to_string()
            on:click=move |_| {
                if interactive {
                    selection.set(Some(TournamentSelection::Player(player)))
                }
            }
            on:keydown=move |event| {
                if interactive && (event.key() == "Enter" || event.key() == " ") {
                    event.prevent_default();
                    selection.set(Some(TournamentSelection::Player(player)));
                }
            }
        >
            <div class=if is_me {
                "flex flex-none justify-center items-center py-2 px-1 w-6 text-sm font-bold tabular-nums text-center text-gray-600 whitespace-nowrap border-l-4 border-pillbug-teal sm:w-8 dark:text-gray-300"
            } else {
                "flex flex-none justify-center items-center py-2 px-1 w-6 text-sm font-bold tabular-nums text-center text-gray-600 whitespace-nowrap sm:w-8 dark:text-gray-300"
            }>{seed.map(|rank| rank.to_string()).unwrap_or_else(|| String::from("—"))}</div>
            <StandingPlayerIdentity
                name
                rating
                withdrawn
                fixed_desktop_width=interactive && !elimination
                inline_desktop_rating=!show_rating
                show_missing_rating=show_rating
            />
            {show_rating
                .then(|| {
                    view! {
                        <div class="hidden flex-none py-2 px-2 text-xs italic tabular-nums text-right text-gray-500 whitespace-nowrap sm:block sm:px-3 dark:text-gray-400 min-w-[4ch]">
                            {account_rating
                                .map(|rating| rating.to_string())
                                .unwrap_or_else(|| String::from("—"))}
                        </div>
                    }
                })}
            {show_results
                .then(|| {
                    view! {
                        <div class=move || {
                            if max_result_count.get() > 15 {
                                "order-last py-2 pr-3 pl-6 min-w-0 text-right basis-full sm:pl-8"
                            } else {
                                "order-last py-1 pr-2 pl-6 min-w-0 text-right sm:flex-1 sm:order-none sm:py-2 sm:px-2 basis-full sm:basis-auto"
                            }
                        }>
                            {move || {
                                let results = results.get();
                                let marker_class = "w-[1.5ch]";
                                let sheet_class = "text-xs gap-y-1 gap-x-0.5";
                                let result_count = results.len();
                                let condensed = result_count > 35 && !results_expanded.get();
                                if results.is_empty() {
                                    view! {
                                        <span class=format!(
                                            "inline-flex justify-center font-mono {marker_class}",
                                        )>"—"</span>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <span class=format!(
                                            "flex w-full flex-wrap items-center justify-end leading-tight font-mono {sheet_class}",
                                        )>
                                            {results
                                                .into_iter()
                                                .take(if condensed { 35 } else { result_count })
                                                .map(|marker| {
                                                    let color = result_marker_color(marker);
                                                    view! {
                                                        <span class=format!(
                                                            "inline-flex shrink-0 justify-center {color} {marker_class}",
                                                        )>{marker}</span>
                                                    }
                                                })
                                                .collect_view()}
                                            {(result_count > 35)
                                                .then(|| {
                                                    view! {
                                                        // TODO: i18n once copy is approved.
                                                        <button
                                                            type="button"
                                                            class="font-sans text-xs ui-text-link"
                                                            on:click=move |event| {
                                                                event.stop_propagation();
                                                                results_expanded.update(|expanded| *expanded = !*expanded);
                                                            }
                                                            on:keydown=move |event| event.stop_propagation()
                                                        >
                                                            {if condensed {
                                                                format!("Show all {result_count}")
                                                            } else {
                                                                "Show fewer".to_string()
                                                            }}
                                                        </button>
                                                    }
                                                })}
                                        </span>
                                    }
                                        .into_any()
                                }
                            }}
                        </div>
                    }
                })}
            {show_final
                .then(|| {
                    view! {
                        <div class=if wide_state {
                            "flex-none py-2 px-3 w-36 max-w-[calc(100%-1.5rem)] overflow-hidden text-xs font-semibold text-left text-ellipsis whitespace-nowrap sm:w-40 sm:max-w-none sm:text-sm"
                        } else {
                            "flex-none ml-auto py-2 px-1 min-w-8 text-lg font-bold tabular-nums text-right whitespace-nowrap sm:px-3 sm:min-w-14"
                        }>{final_value}</div>
                    }
                })}
        </div>
    }
}

#[component]
fn OverviewStandingRow(
    tournament: TournamentState,
    row: OverviewRow,
    is_me: bool,
    interactive: bool,
    max_result_count: Signal<usize>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    if row.arena.is_some() {
        return view! { <ArenaStandingRow tournament row is_me selection /> }.into_any();
    }
    if !interactive {
        return view! { <WaitingStandingRow tournament row is_me max_result_count selection /> }
            .into_any();
    }
    if matches!(tournament.format, TournamentFormatStore::Elimination(_)) {
        return view! { <EliminationStandingRow tournament row is_me max_result_count selection /> }.into_any();
    }
    view! { <ScoredStandingRow tournament row is_me max_result_count selection /> }.into_any()
}

#[component]
fn WaitingStandingRow(
    tournament: TournamentState,
    row: OverviewRow,
    is_me: bool,
    max_result_count: Signal<usize>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    view! {
        <StandardStandingRow
            tournament
            row
            is_me
            show_results=false
            show_rating=true
            show_final=false
            interactive=false
            max_result_count
            selection
        />
    }
}

#[component]
fn ScoredStandingRow(
    tournament: TournamentState,
    row: OverviewRow,
    is_me: bool,
    max_result_count: Signal<usize>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    view! {
        <StandardStandingRow
            tournament
            row
            is_me
            show_results=true
            show_rating=false
            show_final=true
            interactive=true
            max_result_count
            selection
        />
    }
}

#[component]
fn EliminationStandingRow(
    tournament: TournamentState,
    row: OverviewRow,
    is_me: bool,
    max_result_count: Signal<usize>,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let show_final = row.state.is_some();
    view! {
        <StandardStandingRow
            tournament
            row
            is_me
            show_results=false
            show_rating=true
            show_final
            interactive=true
            max_result_count
            selection
        />
    }
}

#[component]
pub fn TournamentOverviewStandings(
    tournament: TournamentState,
    selection: RwSignal<Option<TournamentSelection>>,
    children: Children,
) -> impl IntoView {
    let auth = expect_context::<AuthContext>();
    let search = RwSignal::new(String::new());
    let suggestions_id = StoredValue::new(format!(
        "overview-entrant-suggestions-{}",
        tournament.tournament_id().0
    ));
    let entrant_names = Signal::derive(move || {
        tournament
            .common
            .memberships()
            .get()
            .players
            .values()
            .map(|player| player.username.clone())
            .collect::<Vec<_>>()
    });
    let page = RwSignal::new(1usize);
    let focus_on_me = RwSignal::new(true);
    let viewer = Signal::derive(move || auth.identity.get().and_then(AuthIdentity::user_id));
    let me_player = Signal::derive(move || {
        let viewer = viewer.get()?;
        tournament
            .common
            .memberships()
            .get()
            .players
            .contains_key(&viewer)
            .then_some(viewer)
    });
    let model = Memo::new_with_compare(
        move |_| OverviewStandingsModel::read(tournament),
        |_, _| true,
    );
    let rows = Signal::derive(move || model.with(|model| model.rows.clone()));
    let filtered_rows = Memo::new_with_compare(
        move |_| {
            let needle = search.get().trim().to_lowercase();
            rows.with(|rows| {
                rows.iter()
                    .filter(|row| needle.is_empty() || row.name.to_lowercase().contains(&needle))
                    .cloned()
                    .collect::<Vec<_>>()
            })
        },
        |_, _| true,
    );
    let total = Signal::derive(move || filtered_rows.with(Vec::len));
    let display_total = Signal::derive(move || {
        if search.get().is_empty() {
            model.with(|model| model.active_total)
        } else {
            total.get()
        }
    });
    let struck_total = Signal::derive(move || {
        if !search.get().is_empty() {
            return None;
        }
        model.with(|model| model.struck_total)
    });
    let max_result_count = Signal::derive(move || {
        maximum_result_count(
            tournament.format,
            tournament.common.lifecycle().get().status,
        )
    });
    let started = Signal::derive(move || {
        tournament.common.lifecycle().get().status != TournamentStatus::NotStarted
    });
    let on_page_change = Callback::new(move |next| {
        focus_on_me.set(false);
        page.set(next);
    });
    let on_query_change = Callback::new(move |query| {
        focus_on_me.set(false);
        search.set(query);
        page.set(1);
    });
    Effect::new(move || {
        let last = total.get().div_ceil(PAGE_SIZE).max(1);
        if page.get() > last {
            page.set(last);
        } else if page.get() == 0 {
            page.set(1);
        }
    });
    let me_page = Signal::derive(move || {
        let me = me_player.get()?;
        rows.with(|rows| {
            rows.iter()
                .position(|row| row.player == me)
                .map(|index| index / PAGE_SIZE + 1)
        })
    });
    let me_on_page = Signal::derive(move || {
        search.get().is_empty() && me_page.get().is_some_and(|me_page| me_page == page.get())
    });
    Effect::new(move || {
        if focus_on_me.get() && search.get().is_empty() {
            if let Some(me_page) = me_page.get() {
                page.set(me_page);
            }
        }
    });
    let on_me = Callback::new(move |player| {
        focus_on_me.set(true);
        if let Some(index) =
            rows.with_untracked(|rows| rows.iter().position(|row| row.player == player))
        {
            page.set(index / PAGE_SIZE + 1);
        }
    });

    view! {
        <section class="overflow-hidden min-w-0 ui-panel">
            {children()}
            <Show when=move || {
                tournament.common.lifecycle().get().status == TournamentStatus::Finished
            }>
                <TournamentPodium common=tournament.common format=tournament.format selection />
            </Show>
            <div class="flex flex-wrap gap-2 justify-between items-center py-2 px-2 min-w-0 border-b sm:px-3 border-black/10 dark:border-white/10">
                <TournamentStandingsControls
                    page=page.into()
                    total
                    page_size=PAGE_SIZE
                    query=search.into()
                    on_query_change
                    on_page_change
                    display_total
                    struck_total
                    show_pagination_when_empty=true
                    me_player
                    me_on_page
                    on_me
                    entrant_names
                    suggestions_id=suggestions_id.get_value()
                />
                <div class="flex gap-2 items-center ml-auto">
                    <Show when=move || {
                        search.get().is_empty()
                            && tournament.common.lifecycle().get().status
                                == TournamentStatus::NotStarted
                    }>
                        {move || {
                            let lifecycle = tournament.common.lifecycle().get();
                            lifecycle
                                .seats
                                .map(|capacity| {
                                    // TODO: i18n once copy is approved.
                                    view! {
                                        <span class="text-sm font-semibold tabular-nums text-gray-600 dark:text-gray-300">
                                            {format!(
                                                "{} / {capacity}",
                                                tournament.common.memberships().get().players.len(),
                                            )}
                                        </span>
                                    }
                                })
                        }}
                    </Show>

                </div>
            </div> <div class="min-w-0">
                <Show when=move || total.get() == 0 && !search.get().trim().is_empty()>
                    <div class="flex gap-3 justify-center items-center p-4 text-sm">
                        // TODO: i18n once copy is approved.
                        <span>"No players match your search."</span>
                        // TODO: i18n once copy is approved.
                        <button
                            type="button"
                            class="ui-text-link"
                            on:click=move |_| on_query_change.run(String::new())
                        >
                            "Clear search"
                        </button>
                    </div>
                </Show>
                <div>
                    {move || {
                        let start = page.get().saturating_sub(1).saturating_mul(PAGE_SIZE);
                        filtered_rows
                            .get()
                            .into_iter()
                            .skip(start)
                            .take(PAGE_SIZE)
                            .map(|row| {
                                let player = row.player;
                                let is_me = me_player.get() == Some(player);
                                let interactive = started.get();
                                view! {
                                    <OverviewStandingRow
                                        tournament
                                        row
                                        is_me
                                        interactive
                                        max_result_count
                                        selection
                                    />
                                }
                            })
                            .collect_view()
                    }}
                </div>
            </div>

        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elimination_result(
        player: u128,
        in_contention: bool,
        placement: Option<u32>,
    ) -> EliminationPlayerResultResponse {
        EliminationPlayerResultResponse {
            player: Uuid::from_u128(player),
            in_contention,
            placement,
            exit_stage: None,
        }
    }

    #[test]
    fn elimination_overview_puts_contenders_before_placement_order() {
        let mut results = vec![
            elimination_result(1, false, Some(9)),
            elimination_result(2, true, None),
            elimination_result(3, false, Some(5)),
            elimination_result(4, true, None),
            elimination_result(5, false, Some(9)),
        ];

        sort_elimination_results(&mut results);

        assert_eq!(
            results
                .into_iter()
                .map(|result| result.player)
                .collect::<Vec<_>>(),
            vec![
                Uuid::from_u128(2),
                Uuid::from_u128(4),
                Uuid::from_u128(3),
                Uuid::from_u128(1),
                Uuid::from_u128(5),
            ]
        );
    }
}
