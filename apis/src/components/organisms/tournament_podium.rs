use crate::{
    common::{
        arena_player_rating,
        display_standing_rows,
        primary_score_presentation,
        standings_count,
        standings_value_text,
        StandingsCount,
    },
    components::{atoms::rating::Rating, organisms::tournament_inspector::TournamentSelection},
    i18n::*,
    providers::{
        ArenaStateStoreFields,
        EliminationState,
        EliminationStateStoreFields,
        RoundRobinStateStoreFields,
        SwissStateStoreFields,
        TournamentCommon,
        TournamentCommonStoreFields,
        TournamentFormatState,
    },
    responses::{tournament::rating_clock, TournamentMemberships},
};
use leptos::prelude::*;
use reactive_stores::Store;
use shared_types::{
    tournament::{
        round_robin::PrimaryScore as RoundRobinPrimaryScore,
        standings::Row,
        swiss::{PrimaryScore as DoubleSwissPrimaryScore, System as SwissSystem},
        Format,
        FormatConfig,
    },
    GameSpeed,
};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone)]
struct PodiumPlayer {
    id: Uuid,
    name: String,
    rating: Option<u64>,
    seed: Option<usize>,
    score: Option<String>,
    performance: Option<i32>,
    wins: u32,
    draws: u32,
    losses: u32,
    berserks: u32,
}

#[derive(Clone)]
struct PodiumPlacement {
    rank: u32,
    players: Vec<PodiumPlayer>,
}

fn podium_record(configuration: &FormatConfig, row: &Row) -> (u32, u32, u32) {
    if matches!(configuration, FormatConfig::RoundRobin(config) if config.rules.primary_score == RoundRobinPrimaryScore::MatchPoints)
    {
        return (
            standings_count(row, StandingsCount::RoundRobinMatchWins),
            standings_count(row, StandingsCount::RoundRobinMatchDraws),
            standings_count(row, StandingsCount::RoundRobinMatchLosses),
        );
    }
    let FormatConfig::Swiss(configuration) = configuration else {
        return (row.wins, row.draws, row.losses);
    };
    let SwissSystem::DoubleSwiss(configuration) = &configuration.system else {
        return (row.wins, row.draws, row.losses);
    };
    if configuration.primary_score != DoubleSwissPrimaryScore::MatchPoints {
        return (row.wins, row.draws, row.losses);
    }
    (
        standings_count(row, StandingsCount::SwissMatchWins),
        standings_count(row, StandingsCount::SwissMatchDraws),
        standings_count(row, StandingsCount::SwissMatchLosses),
    )
}

fn standing_podium(
    common: Store<TournamentCommon>,
    format: TournamentFormatState,
    configuration: &FormatConfig,
) -> Vec<PodiumPlacement> {
    let memberships = common.memberships().get();
    let standings = common.standings().get();
    let speed = rating_clock(configuration).map(GameSpeed::from);
    let presentation = primary_score_presentation(configuration);
    let arena = match format {
        TournamentFormatState::Arena(state) => Some(state),
        _ => None,
    };
    let mut players = Vec::new();
    for standing in standings
        .snapshot
        .iter()
        .flat_map(|standings| display_standing_rows(standings, &memberships.withdrawn))
    {
        let Some(rank) = standing.rank else {
            continue;
        };
        if rank > 3 {
            continue;
        }
        let row = &standing.row;
        let Some(user) = memberships.players.get(&row.user_id) else {
            continue;
        };
        let player_arena_stats = arena.and_then(|arena| {
            arena
                .player_stats()
                .with_untracked(|stats| stats.contains_key(&row.user_id))
                .then(|| arena.player_stats().at_key(row.user_id).get())
        });
        let (wins, draws, losses) = player_arena_stats.as_ref().map_or_else(
            || podium_record(configuration, row),
            |stats| (stats.wins, stats.draws, stats.losses),
        );
        players.push((
            rank,
            PodiumPlayer {
                id: user.uid,
                name: user.username.clone(),
                rating: match player_arena_stats.as_ref() {
                    Some(stats) => arena_player_rating(stats),
                    None if arena.is_none() => {
                        speed.as_ref().map(|speed| user.rating_for_speed(speed))
                    }
                    None => None,
                },
                seed: memberships
                    .pairing_numbers
                    .get(&row.user_id)
                    .map(|seed| seed + 1),
                score: Some(standings_value_text(row.primary_score, presentation)),
                performance: player_arena_stats
                    .as_ref()
                    .and_then(|stats| stats.performance_rating),
                wins,
                draws,
                losses,
                berserks: player_arena_stats
                    .as_ref()
                    .map_or(0, |stats| stats.berserks),
            },
        ));
    }

    group_podium_players(players)
}

fn group_podium_players(
    players: impl IntoIterator<Item = (u32, PodiumPlayer)>,
) -> Vec<PodiumPlacement> {
    let mut groups = BTreeMap::<u32, Vec<PodiumPlayer>>::new();
    for (rank, player) in players {
        if (1..=3).contains(&rank) {
            groups.entry(rank).or_default().push(player);
        }
    }
    groups
        .into_iter()
        .map(|(rank, players)| PodiumPlacement { rank, players })
        .collect()
}

fn elimination_player(
    memberships: &TournamentMemberships,
    player: Uuid,
    speed: Option<&GameSpeed>,
) -> Option<PodiumPlayer> {
    let user = memberships.players.get(&player)?;
    Some(PodiumPlayer {
        id: user.uid,
        name: user.username.clone(),
        rating: speed.map(|speed| user.rating_for_speed(speed)),
        seed: memberships
            .pairing_numbers
            .get(&player)
            .map(|seed| seed + 1),
        score: None,
        performance: None,
        wins: 0,
        draws: 0,
        losses: 0,
        berserks: 0,
    })
}

fn elimination_podium(
    common: Store<TournamentCommon>,
    elimination: Store<EliminationState>,
    configuration: &FormatConfig,
) -> Vec<PodiumPlacement> {
    let memberships = common.memberships().get();
    let speed = rating_clock(configuration).map(GameSpeed::from);
    let player_results = elimination.player_results().get();
    group_podium_players(player_results.iter().filter_map(|result| {
        let rank = result.placement?;
        if memberships.withdrawn.contains(&result.player) {
            return None;
        }
        Some((
            rank,
            elimination_player(&memberships, result.player, speed.as_ref())?,
        ))
    }))
}

fn podium_placements(
    common: Store<TournamentCommon>,
    format: TournamentFormatState,
) -> Vec<PodiumPlacement> {
    match format {
        TournamentFormatState::Arena(state) => {
            let configuration = FormatConfig::Arena(state.configuration().get_untracked());
            standing_podium(common, format, &configuration)
        }
        TournamentFormatState::RoundRobin(state) => {
            let configuration = FormatConfig::RoundRobin(state.configuration().get_untracked());
            standing_podium(common, format, &configuration)
        }
        TournamentFormatState::Swiss(state) => {
            let configuration = FormatConfig::Swiss(state.configuration().get_untracked());
            standing_podium(common, format, &configuration)
        }
        TournamentFormatState::Elimination(state) => {
            let configuration = FormatConfig::Elimination(state.configuration().get_untracked());
            elimination_podium(common, state, &configuration)
        }
    }
}

fn placement_label(rank: u32, shared: bool) -> String {
    let i18n = use_i18n();
    match (rank, shared) {
        (1, false) => {
            t_string!(i18n, tournaments.creation.elimination.colors.first_short).to_string()
        }
        (1, true) => t_string!(i18n, tournaments.ui.joint_first).to_string(),
        (2, false) => {
            t_string!(i18n, tournaments.creation.elimination.colors.second_short).to_string()
        }
        (2, true) => t_string!(i18n, tournaments.ui.joint_second).to_string(),
        (3, false) => t_string!(i18n, common.third).to_string(),
        (3, true) => t_string!(i18n, tournaments.ui.joint_third).to_string(),
        _ => t_string!(i18n, tournaments.ui.placed).to_string(),
    }
}

fn player_detail(player: &PodiumPlayer, format: Format) -> String {
    let i18n = use_i18n();
    if matches!(
        format,
        Format::SingleElimination | Format::DoubleElimination
    ) {
        return player
            .seed
            .map(|seed| t_string!(i18n, tournaments.ui.seed, seed = seed).to_string())
            .unwrap_or_else(|| t_string!(i18n, common.seed).to_string());
    }
    let record = t_string!(
        i18n,
        tournaments.ui.w_d_l,
        value1 = player.wins,
        value2 = player.draws,
        value3 = player.losses
    )
    .to_string();
    if format == Format::Arena {
        let performance = player
            .performance
            .map(|performance| {
                t_string!(i18n, tournaments.ui.perf, performance = performance).to_string()
            })
            .unwrap_or_default();
        return if player.berserks == 0 {
            format!("{performance}{record}")
        } else {
            t_string!(
                i18n,
                tournaments.ui.berserks,
                performance = performance,
                record = record,
                value1 = player.berserks
            )
            .to_string()
        };
    }
    record
}

#[component]
pub fn TournamentPodium(
    common: Store<TournamentCommon>,
    format: TournamentFormatState,
    selection: RwSignal<Option<TournamentSelection>>,
) -> impl IntoView {
    let i18n = use_i18n();
    let placements =
        Memo::new_with_compare(move |_| podium_placements(common, format), |_, _| true);
    let format = format.format();
    view! {
        <section
            class="p-3 border-b border-black/10 dark:border-white/10"
            aria-label=move || t_string!(i18n, tournaments.ui.tournament_podium).to_string()
            data-testid="tournament-podium"
        >
            {move || {
                placements
                    .get()
                    .into_iter()
                    .map(|placement| {
                        let count = placement.players.len();
                        let expanded = RwSignal::new(false);
                        let rank = placement.rank;
                        view! {
                            <div
                                class="flex gap-3 items-start py-1.5 border-b last:border-0 border-black/5 dark:border-white/10"
                                data-podium-rank=rank
                            >
                                <svg
                                    class="tournament-podium-place"
                                    data-rank=rank
                                    viewBox="0 0 34 39"
                                    aria-hidden="true"
                                >
                                    <polygon
                                        points="17,1.5 32.6,10.5 32.6,28.5 17,37.5 1.4,28.5 1.4,10.5"
                                        stroke-width="1.5"
                                        stroke-linejoin="round"
                                    />
                                    <text
                                        x="17"
                                        y="24"
                                        text-anchor="middle"
                                        fill="currentColor"
                                        stroke="none"
                                    >
                                        {rank}
                                    </text>
                                </svg>
                                <div class="grid flex-1 gap-2 items-center min-w-0 grid-cols-[4rem_minmax(0,1fr)] min-h-9">
                                    <strong class="text-sm">
                                        {placement_label(rank, count > 1)}
                                    </strong>
                                    <div class="min-w-0">
                                        {(count > 6)
                                            .then(|| {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="text-sm ui-text-link"
                                                        on:click=move |_| expanded.update(|value| *value = !*value)
                                                    >
                                                        {move || {
                                                            if expanded.get() {
                                                                t_string!(i18n, tournaments.ui.hide_players).to_string()
                                                            } else {
                                                                t_string!(
                                                                    i18n, tournaments.ui.show_all_players, count = count
                                                                )
                                                                    .to_string()
                                                            }
                                                        }}
                                                    </button>
                                                }
                                            })}
                                        <div
                                            class="flex flex-wrap gap-2 min-w-0"
                                            class:hidden=move || { count > 6 && !expanded.get() }
                                        >
                                            {placement
                                                .players
                                                .into_iter()
                                                .map(|player| {
                                                    let player_id = player.id;
                                                    let detail = player_detail(&player, format);
                                                    view! {
                                                        <button
                                                            type="button"
                                                            class="min-w-0 max-w-full ui-button ui-button-secondary ui-button-sm"
                                                            title=detail
                                                            data-podium-player-id=player_id.to_string()
                                                            on:click=move |_| {
                                                                selection.set(Some(TournamentSelection::Player(player_id)))
                                                            }
                                                        >
                                                            <span class="truncate">{player.name}</span>
                                                            {player
                                                                .score
                                                                .map(|score| {
                                                                    view! {
                                                                        <strong class="tabular-nums shrink-0">" · "{score}</strong>
                                                                    }
                                                                })}
                                                            {player
                                                                .rating
                                                                .map(|rating| {
                                                                    view! {
                                                                        <span class="text-xs italic text-gray-500 dark:text-gray-400 shrink-0">
                                                                            <Rating value=rating />
                                                                        </span>
                                                                    }
                                                                })}
                                                        </button>
                                                    }
                                                })
                                                .collect_view()}
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn podium_preserves_official_places_and_every_player_at_those_places() {
        for ranks in [
            vec![1, 1, 3],
            vec![1, 2, 3, 3],
            vec![1, 1, 1, 1],
            vec![1, 2, 3, 4],
        ] {
            let placements = group_podium_players(ranks.iter().enumerate().map(|(index, rank)| {
                (
                    *rank,
                    PodiumPlayer {
                        id: Uuid::from_u128(index as u128),
                        name: index.to_string(),
                        rating: None,
                        seed: None,
                        score: Some("10".to_string()),
                        performance: None,
                        wins: 0,
                        draws: 0,
                        losses: 0,
                        berserks: 0,
                    },
                )
            }));
            let actual = placements
                .iter()
                .flat_map(|placement| {
                    placement
                        .players
                        .iter()
                        .map(move |player| (placement.rank, player.id))
                })
                .collect::<Vec<_>>();
            let expected = ranks
                .into_iter()
                .enumerate()
                .filter(|(_, rank)| *rank <= 3)
                .map(|(index, rank)| (rank, Uuid::from_u128(index as u128)))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected);
        }
    }
}
