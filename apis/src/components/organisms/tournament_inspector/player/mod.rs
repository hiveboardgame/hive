use crate::responses::tournament::rating_clock;
use shared_types::tournament_view::{
    ArenaGameResponse,
    EliminationNodeStateResponse,
    EliminationPlayerResultResponse,
    SlotResponse,
};
mod arena;
mod standard;

use self::{
    arena::ArenaPlayerDetailsBody,
    standard::{EliminationPlayerDetailsBody, RoundRobinPlayerDetailsBody, SwissPlayerDetailsBody},
};
use super::TournamentSelection;
use crate::{
    common::{
        game_point_presentation,
        primary_value_text,
        standings_value_text,
        ScorePresentation,
    },
    components::{atoms::color_hex::ColorHex, organisms::elimination_result::result_label},
    i18n::*,
    providers::{
        ArenaState,
        ArenaStateStoreFields,
        EliminationState,
        EliminationStateStoreFields,
        RoundRobinState,
        RoundRobinStateStoreFields,
        SwissState,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
        TournamentFormatStore,
        TournamentState,
    },
    responses::{TournamentMemberships, TournamentStandings},
};
use hive_lib::Color;
use leptos::prelude::*;
use leptos_icons::Icon;
use reactive_stores::Store;
use shared_types::{
    tournament::{
        elimination::Resolution as EliminationNodeResolution,
        round_robin::PrimaryScore as RoundRobinPrimaryScore,
        standings::{Row, Value},
        swiss::{PrimaryScore as DoubleSwissPrimaryScore, System as SwissSystem},
        AdjudicatedSideResult,
        FormatConfig,
        GameOutcome,
        PlayedGameOutcome,
        Resolution,
        Score,
    },
    GameId,
    GameSpeed,
};
use tournamint::MatchScore;
use uuid::Uuid;

#[derive(Clone)]
struct PlayerStatistics {
    performance: Option<i32>,
    record: String,
    win_rate: Option<String>,
    average_opponent: Option<u32>,
}

#[derive(Clone, PartialEq)]
struct HistoryGame {
    order: usize,
    game_id: Option<GameId>,
    color: Option<Color>,
    board_result: String,
    selected_score: String,
    result: Option<MatchScore>,
    state: Option<&'static str>,
}

#[derive(Clone, PartialEq)]
struct OpponentLabel {
    name: String,
    rating: Option<u64>,
}

const PLAYER_HISTORY_ROW_SHELL: &str = "items-center gap-1 px-1 py-1.5 text-sm odd:bg-odd-light even:bg-even-light hover:bg-blue-light/70 dark:odd:bg-surface-row-odd dark:even:bg-surface-row-even dark:hover:bg-pillbug-teal/15";
const PLAYER_HISTORY_GROUP_SHELL: &str = "group text-sm odd:bg-odd-light even:bg-even-light dark:odd:bg-surface-row-odd dark:even:bg-surface-row-even";

fn player_history_row_class(columns: &str) -> String {
    format!("grid {columns} {PLAYER_HISTORY_ROW_SHELL}")
}

fn player_name(memberships: &TournamentMemberships, player: Uuid) -> String {
    memberships
        .players
        .get(&player)
        .map(|user| user.username.clone())
        .unwrap_or_else(|| String::from("—"))
}

pub(crate) fn player_standing(standings: &TournamentStandings, player: Uuid) -> Option<(u32, Row)> {
    standings
        .snapshot
        .as_ref()?
        .groups
        .iter()
        .find_map(|group| {
            group
                .rows
                .iter()
                .find(|row| row.user_id == player)
                .cloned()
                .map(|row| (group.placement.rank(), row))
        })
}

fn percentage(numerator: u32, denominator: u32) -> Option<String> {
    (denominator != 0).then(|| {
        let rounded = numerator
            .saturating_mul(100)
            .saturating_add(denominator / 2)
            / denominator;
        format!("{rounded}%")
    })
}

fn player_statistics(common: Store<TournamentCommon>, player: Uuid) -> PlayerStatistics {
    common
        .standings()
        .get()
        .player_stats
        .iter()
        .find(|stats| stats.player == player)
        .map(|stats| {
            let games = stats
                .wins
                .saturating_add(stats.draws)
                .saturating_add(stats.losses);
            PlayerStatistics {
                performance: stats.performance_rating,
                record: format!("{}–{}–{}", stats.wins, stats.draws, stats.losses,),
                win_rate: percentage(stats.wins, games),
                average_opponent: stats.average_opponent_rating,
            }
        })
        .unwrap_or_else(|| PlayerStatistics {
            performance: None,
            record: String::from("0–0–0"),
            win_rate: None,
            average_opponent: None,
        })
}

#[component]
fn PlayerStatisticsTable(
    statistics: PlayerStatistics,
    berserk_rate: Option<String>,
) -> impl IntoView {
    let PlayerStatistics {
        performance,
        record,
        win_rate,
        average_opponent,
    } = statistics;
    // TODO: i18n once copy is approved.
    view! {
        <dl class="grid gap-x-4 px-1 text-sm grid-cols-[minmax(0,1fr)_auto]">
            {performance
                .map(|performance| {
                    view! {
                        <dt class="py-0.5 text-gray-600 dark:text-gray-300">"Performance"</dt>
                        <dd class="py-0.5 font-semibold tabular-nums text-right">{performance}</dd>
                    }
                })} <dt class="py-0.5 text-gray-600 dark:text-gray-300">"W–D–L"</dt>
            <dd class="py-0.5 font-semibold tabular-nums text-right">{record}</dd>
            {win_rate
                .map(|win_rate| {
                    view! {
                        <dt class="py-0.5 text-gray-600 dark:text-gray-300">"Win rate"</dt>
                        <dd class="py-0.5 font-semibold tabular-nums text-right">{win_rate}</dd>
                    }
                })}
            {berserk_rate
                .map(|berserk_rate| {
                    view! {
                        <dt class="py-0.5 text-gray-600 dark:text-gray-300">"Berserk rate"</dt>
                        <dd class="py-0.5 font-semibold tabular-nums text-right">{berserk_rate}</dd>
                    }
                })}
            {average_opponent
                .map(|average_opponent| {
                    view! {
                        <dt class="py-0.5 text-gray-600 dark:text-gray-300">"Average opponent"</dt>
                        <dd class="py-0.5 font-semibold tabular-nums text-right">
                            {average_opponent}
                        </dd>
                    }
                })}
        </dl>
    }
}

fn slot_side(slot: &SlotResponse, player: Uuid) -> Option<usize> {
    slot.participants
        .iter()
        .position(|participant| *participant == player)
}

fn slot_opponent(slot: &SlotResponse, player: Uuid) -> Option<Uuid> {
    let side = slot_side(slot, player)?;
    Some(slot.participants[1 - side])
}

fn opponent_rating(
    common: Store<TournamentCommon>,
    configuration: &FormatConfig,
    opponent: Uuid,
) -> Option<u64> {
    let speed = rating_clock(configuration).map(GameSpeed::from)?;
    common
        .memberships()
        .get()
        .players
        .get(&opponent)
        .and_then(|user| user.ratings.get(&speed))
        .map(|rating| rating.rating)
}

fn opponent_label(
    common: Store<TournamentCommon>,
    configuration: &FormatConfig,
    opponent: Option<Uuid>,
) -> OpponentLabel {
    let Some(opponent) = opponent else {
        // TODO: i18n once copy is approved.
        return OpponentLabel {
            name: String::from("Bye"),
            rating: None,
        };
    };
    OpponentLabel {
        name: player_name(&common.memberships().get(), opponent),
        rating: opponent_rating(common, configuration, opponent),
    }
}

fn board_result_text(outcome: GameOutcome) -> &'static str {
    match outcome {
        GameOutcome::Played(PlayedGameOutcome::WhiteWin) => "1–0",
        GameOutcome::Played(PlayedGameOutcome::Draw) => "½–½",
        GameOutcome::Played(PlayedGameOutcome::BlackWin) => "0–1",
        GameOutcome::Adjudicated(outcome) => match (outcome.white(), outcome.black()) {
            (AdjudicatedSideResult::ForfeitWin, AdjudicatedSideResult::ForfeitLoss) => "1–0",
            (AdjudicatedSideResult::ForfeitLoss, AdjudicatedSideResult::ForfeitWin) => "0–1",
            (AdjudicatedSideResult::Draw, AdjudicatedSideResult::Draw) => "½–½",
            (AdjudicatedSideResult::Draw, AdjudicatedSideResult::ForfeitLoss) => "½–0",
            (AdjudicatedSideResult::ForfeitLoss, AdjudicatedSideResult::Draw) => "0–½",
            (AdjudicatedSideResult::DoubleForfeit, AdjudicatedSideResult::DoubleForfeit) => "0–0",
            _ => unreachable!("validated adjudicated outcome"),
        },
    }
}

fn player_match_score(outcome: GameOutcome, side: usize) -> MatchScore {
    if side == 0 {
        outcome.white().score()
    } else {
        outcome.black().score()
    }
}

fn game_points_text(configuration: &FormatConfig, points: Score) -> String {
    let presentation = match configuration {
        FormatConfig::RoundRobin(configuration) => {
            game_point_presentation(configuration.game_point_system)
        }
        FormatConfig::Swiss(configuration) => {
            game_point_presentation(configuration.game_point_system)
        }
        FormatConfig::Arena(_) => ScorePresentation::ArenaPoints,
        FormatConfig::Elimination(_) => ScorePresentation::Integer,
    };
    standings_value_text(Value::Score(points), presentation)
}

fn primary_score_text(configuration: &FormatConfig, points: Score) -> String {
    primary_value_text(Value::Score(points), configuration)
}

// TODO: i18n once copy is approved.
fn slot_state(slot: &SlotResponse) -> Option<&'static str> {
    match slot.resolution {
        Some(Resolution::Withdrawal(_)) => Some("Withdrawn"),
        Some(Resolution::Clinched) => Some("Not needed"),
        Some(Resolution::Result(GameOutcome::Adjudicated(outcome)))
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
        Some(Resolution::Result(GameOutcome::Adjudicated(_))) => Some("Adjudicated"),
        Some(Resolution::Result(GameOutcome::Played(_))) => None,
        None if slot.game.as_ref().is_some_and(|game| !game.finished) => Some("Playing"),
        None if slot.game.is_none() => Some("Planned"),
        None => None,
    }
}

fn slot_history_game(
    configuration: &FormatConfig,
    slot: &SlotResponse,
    player: Uuid,
    order: usize,
) -> HistoryGame {
    let side = slot_side(slot, player).unwrap_or_default();
    let outcome = slot.outcome();
    let result = outcome.map(|outcome| player_match_score(outcome, side));
    let selected_score = slot
        .awarded_game_points
        .map(|points| game_points_text(configuration, points[side]))
        .unwrap_or_else(|| {
            result.map_or_else(
                || {
                    if slot.game.as_ref().is_some_and(|game| !game.finished) {
                        String::from("*")
                    } else {
                        String::from("—")
                    }
                },
                |score| match score {
                    MatchScore::Win => String::from("1"),
                    MatchScore::Draw => String::from("½"),
                    MatchScore::Loss => String::from("0"),
                },
            )
        });
    HistoryGame {
        order,
        game_id: slot.game_id().cloned(),
        color: (slot.resolution != Some(Resolution::Clinched)).then_some(if side == 0 {
            Color::White
        } else {
            Color::Black
        }),
        board_result: outcome.map(board_result_text).unwrap_or("—").to_string(),
        selected_score,
        result,
        state: slot_state(slot),
    }
}

fn empty_history_game(selected_score: String, state: Option<&'static str>) -> HistoryGame {
    HistoryGame {
        order: usize::MAX,
        game_id: None,
        color: None,
        board_result: String::from("—"),
        selected_score,
        result: None,
        state,
    }
}

#[derive(Clone, PartialEq)]
struct PlayerHeaderDetails {
    title: String,
    profile_href: String,
    badge: Option<String>,
    metadata: String,
}

fn player_header_details(
    common: Store<TournamentCommon>,
    configuration: &FormatConfig,
    elimination_result: Option<EliminationPlayerResultResponse>,
    player: Uuid,
) -> PlayerHeaderDetails {
    let memberships = common.memberships().get();
    let standings = common.standings().get();
    let name = player_name(&memberships, player);
    let standing = player_standing(&standings, player);
    let is_elimination = matches!(configuration, FormatConfig::Elimination(_));
    let title = if is_elimination {
        name.clone()
    } else {
        standing
            .as_ref()
            .map_or_else(|| name.clone(), |(rank, _)| format!("#{rank} {name}"))
    };
    let rating = rating_clock(configuration)
        .map(GameSpeed::from)
        .and_then(|speed| {
            memberships
                .players
                .get(&player)
                .map(|player| player.rating_for_speed(&speed))
        });
    let mut metadata = Vec::new();
    if let Some(rating) = rating {
        metadata.push(rating.to_string());
    }
    match configuration {
        FormatConfig::Elimination(_) => {
            if let Some(seed) = memberships
                .pairing_numbers
                .get(&player)
                .map(|seed| seed.saturating_add(1))
            {
                // TODO: i18n once copy is approved.
                metadata.push(format!("seed {seed}"));
            }
        }
        FormatConfig::RoundRobin(_) | FormatConfig::Arena(_) => {
            if let Some((_, row)) = standing.as_ref() {
                let score = primary_value_text(row.primary_score, configuration);
                // TODO: i18n once copy is approved.
                let suffix = if matches!(configuration, FormatConfig::RoundRobin(config)
                    if config.primary_score == RoundRobinPrimaryScore::MatchPoints)
                {
                    "match pts"
                } else {
                    "pts"
                };
                metadata.push(format!("{score} {suffix}"));
            }
        }
        FormatConfig::Swiss(swiss) => {
            if let Some((_, row)) = standing.as_ref() {
                let score = primary_value_text(row.primary_score, configuration);
                // TODO: i18n once copy is approved.
                let suffix = if matches!(
                    swiss.system,
                    SwissSystem::DoubleSwiss(system)
                        if system.primary_score == DoubleSwissPrimaryScore::MatchPoints
                ) {
                    "match pts"
                } else {
                    "pts"
                };
                metadata.push(format!("{score} {suffix}"));
            }
        }
    }
    PlayerHeaderDetails {
        title,
        profile_href: format!("/@/{name}"),
        badge: elimination_result.and_then(result_label),
        metadata: metadata.join(" · "),
    }
}

#[component]
pub(super) fn TournamentPlayerDetails(
    tournament: TournamentState,
    player: Uuid,
    selection: RwSignal<Option<TournamentSelection>>,
    close: Callback<()>,
) -> impl IntoView {
    let common = tournament.common;
    let (details, body) = match tournament.format {
        TournamentFormatStore::Arena(arena) => (
            Memo::new(move |_| {
                let configuration = FormatConfig::Arena(arena.configuration().get());
                player_header_details(common, &configuration, None, player)
            }),
            view! { <ArenaPlayerDetailsBody common arena player selection /> }.into_any(),
        ),
        TournamentFormatStore::RoundRobin(round_robin) => (
            Memo::new(move |_| {
                let configuration = FormatConfig::RoundRobin(round_robin.configuration().get());
                player_header_details(common, &configuration, None, player)
            }),
            view! { <RoundRobinPlayerDetailsBody common round_robin player /> }.into_any(),
        ),
        TournamentFormatStore::Swiss(swiss) => (
            Memo::new(move |_| {
                let configuration = FormatConfig::Swiss(swiss.configuration().get());
                player_header_details(common, &configuration, None, player)
            }),
            view! { <SwissPlayerDetailsBody common swiss player /> }.into_any(),
        ),
        TournamentFormatStore::Elimination(elimination) => (
            Memo::new(move |_| {
                let configuration = FormatConfig::Elimination(elimination.configuration().get());
                let result = elimination
                    .player_results()
                    .get()
                    .iter()
                    .find(|result| result.player == player)
                    .copied();
                player_header_details(common, &configuration, result, player)
            }),
            view! { <EliminationPlayerDetailsBody common elimination player /> }.into_any(),
        ),
    };
    view! {
        <div>
            <header class="flex sticky top-0 z-20 gap-2 justify-between items-start p-3 border-b border-black/10 bg-even-light dark:border-white/10 dark:bg-surface-panel">
                <div class="min-w-0">
                    {move || {
                        let details = details.get();
                        view! {
                            <div class="flex flex-wrap gap-2 items-center">
                                <h2 class="text-lg font-bold truncate">
                                    <a
                                        class="no-link-style hover:text-pillbug-teal"
                                        href=details.profile_href
                                    >
                                        {details.title}
                                    </a>
                                </h2>
                                {details
                                    .badge
                                    .map(|badge| {
                                        view! {
                                            <span class="py-0.5 px-2 text-xs font-semibold rounded border text-pillbug-teal border-pillbug-teal/40 bg-pillbug-teal/10">
                                                {badge}
                                            </span>
                                        }
                                    })}
                            </div>
                            {(!details.metadata.is_empty())
                                .then(|| {
                                    view! {
                                        <p class="text-xs tabular-nums text-gray-600 dark:text-gray-300">
                                            {details.metadata}
                                        </p>
                                    }
                                })}
                        }
                    }}
                </div>
                <button
                    type="button"
                    class="inline-flex justify-center items-center text-gray-400 rounded transition size-8 shrink-0 dark:hover:bg-white/10 hover:bg-black/5 hover:text-ladybug-red focus-visible:text-ladybug-red"
                    on:click=move |_| close.run(())
                >
                    <Icon icon=icondata_io::IoCloseSharp attr:class="size-5" />
                </button>
            </header>
            <div class="p-3">{body}</div>
        </div>
    }
    .into_any()
}
